// I/O module — open, save, and export CAD documents.
//
// All file reading/writing goes through acadrust.
// Default save format: DWG (AC1032 / R2018+).

pub mod obj;
pub mod pdf_export;
pub mod plot_style;
pub mod print_to_printer;
pub mod step;
pub mod stl;
pub mod xref;

use crate::scene::DerivedCaches;
use acadrust::entities::{Dimension, EntityType};
use acadrust::io::dwg::DwgReader;
use acadrust::{CadDocument, DwgWriter, DxfReader, DxfWriter};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

// Phase tags written into the shared atomic so the UI overlay can display a
// human-readable label. Kept in sync with the constants in `crate::app`.
const PHASE_PARSING: u8 = 1;
const PHASE_CACHING: u8 = 2;
const PHASE_FINALIZING: u8 = 3;

// ── Open ──────────────────────────────────────────────────────────────────

/// Show the file picker and return the chosen path plus its size in bytes.
/// Returning size up-front lets the loading overlay display "47.3 MB" before
/// the parser thread starts.
pub async fn pick_open_path() -> Option<(PathBuf, u64)> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Open CAD file")
        .add_filter("CAD Files", &["dwg", "dxf", "DWG", "DXF"])
        .add_filter("DWG Files", &["dwg", "DWG"])
        .add_filter("DXF Files", &["dxf", "DXF"])
        .add_filter("All Files", &["*"])
        .pick_file()
        .await?;
    let path = handle.path().to_path_buf();
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    Some((path, size))
}

/// Load a CAD file from a known path. Parsing and cache building run on a
/// dedicated OS thread so the async executor stays free for rendering during
/// the load. Writes phase markers into `phase` so the UI can show
/// "Parsing entities…" / "Building caches…" / "Finalizing…" while the loader
/// thread runs.
pub async fn open_path_with_phase(
    path: PathBuf,
    phase: Arc<AtomicU8>,
) -> Result<(String, PathBuf, CadDocument, DerivedCaches), String> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".into());
    let path2 = path.clone();
    let phase2 = phase.clone();
    let (doc, caches) = std::thread::spawn(move || -> Result<_, String> {
        use std::time::Instant;
        phase2.store(PHASE_PARSING, Ordering::Relaxed);
        let t_parse = Instant::now();
        let mut doc = load_file(&path2)?;
        let parse_ms = t_parse.elapsed().as_millis() as u32;
        let t_purge = Instant::now();
        let dropped = purge_corrupt_entities(&mut doc);
        let purge_ms = t_purge.elapsed().as_millis() as u32;
        phase2.store(PHASE_CACHING, Ordering::Relaxed);
        let t_caches = Instant::now();
        let mut caches = crate::scene::build_derived_caches(&doc);
        caches.timings = crate::scene::OpenTimings {
            parse_ms,
            purge_ms,
            caches_ms: t_caches.elapsed().as_millis() as u32,
        };
        caches.corrupt_dropped = dropped;
        phase2.store(PHASE_FINALIZING, Ordering::Relaxed);
        Ok((doc, caches))
    })
    .join()
    .map_err(|_| "parser thread panicked".to_string())??;
    Ok((name, path, doc, caches))
}

/// Load a DWG or DXF file directly from a path (auto-detect by extension).
pub fn load_file(path: &Path) -> Result<CadDocument, String> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "dwg" => {
            let mut doc = DwgReader::from_file(path)
                .map_err(|e| e.to_string())?
                .read()
                .map_err(|e| e.to_string())?;
            fix_viewport_status_flags(&mut doc);
            fix_current_style_names(&mut doc);
            Ok(doc)
        }
        "dxf" => {
            let mut doc = DxfReader::from_file(path)
                .map_err(|e| e.to_string())?
                .read()
                .map_err(|e| e.to_string())?;
            fix_dxf_dimension_rotations(&mut doc);
            fix_viewport_status_flags(&mut doc);
            fix_current_style_names(&mut doc);
            Ok(doc)
        }
        _ => Err(format!("Unsupported file format: .{ext}")),
    }
}

// ── Directory listing (used by the custom Save As dialog) ────────────────

/// Read `dir` and return sorted entries: `(display_name, is_dir, full_path)`.
/// Directories come first, then files.  Hidden entries (`.`) are skipped.
pub fn read_dir_entries(dir: &std::path::Path) -> Vec<(String, bool, PathBuf)> {
    let mut dirs: Vec<(String, bool, PathBuf)> = Vec::new();
    let mut files: Vec<(String, bool, PathBuf)> = Vec::new();

    if let Some(parent) = dir.parent() {
        dirs.push(("..".to_string(), true, parent.to_path_buf()));
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir {
                dirs.push((name, true, entry.path()));
            } else {
                files.push((name, false, entry.path()));
            }
        }
    }
    dirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    files.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    dirs.extend(files);
    dirs
}

// ── Save ──────────────────────────────────────────────────────────────────

/// Parse a format string like "DWG 2013" or "DXF 2007" into
/// `(extension, DxfVersion)`.  Falls back to ("dwg", AC1032) for unknown strings.
pub fn parse_save_format(format: &str) -> (&'static str, acadrust::DxfVersion) {
    use acadrust::DxfVersion;
    let f = format.to_ascii_uppercase();
    let is_dxf = f.starts_with("DXF");
    let ext = if is_dxf { "dxf" } else { "dwg" };
    let version = if f.contains("2013") {
        DxfVersion::AC1027
    } else if f.contains("2010") {
        DxfVersion::AC1024
    } else if f.contains("2007") {
        DxfVersion::AC1021
    } else if f.contains("2004") {
        DxfVersion::AC1018
    } else if f.contains("2000") {
        DxfVersion::AC1015
    } else if f.contains("R14") {
        DxfVersion::AC1014
    } else {
        DxfVersion::AC1032
    }; // 2018
    (ext, version)
}

// ── Plot Style Table ──────────────────────────────────────────────────────

/// Show a file-open dialog and load the selected CTB or STB file.
pub async fn pick_plot_style() -> Option<plot_style::PlotStyleTable> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Load Plot Style Table")
        .add_filter("Plot Style Tables", &["ctb", "stb", "CTB", "STB"])
        .add_filter("CTB Files", &["ctb", "CTB"])
        .add_filter("STB Files", &["stb", "STB"])
        .add_filter("All Files", &["*"])
        .pick_file()
        .await?;
    plot_style::PlotStyleTable::load(handle.path()).ok()
}

// ── Image file picker ─────────────────────────────────────────────────────

/// Show a file-open dialog for raster images and decode the selected file.
/// Returns `(path, pixel_width, pixel_height)` or an error string.
pub async fn pick_image_file() -> Result<(PathBuf, u32, u32), String> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Select Image File")
        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "tiff", "tif"])
        .add_filter("PNG", &["png"])
        .add_filter("JPEG", &["jpg", "jpeg"])
        .add_filter("All Files", &["*"])
        .pick_file()
        .await
        .ok_or_else(|| "Cancelled".to_string())?;
    let path = handle.path().to_path_buf();
    let img = image::open(&path).map_err(|e| e.to_string())?;
    let (w, h) = image::GenericImageView::dimensions(&img);
    Ok((path, w, h))
}

/// Save `doc` to `path` with the given DXF version, overriding `doc.version`.
/// Format is auto-detected from the extension (dwg / dxf).
pub fn save_as_version(
    doc: &CadDocument,
    path: &Path,
    version: acadrust::DxfVersion,
) -> Result<(), String> {
    let mut doc = doc.clone();
    doc.version = version;
    sync_current_style_vardicts(&mut doc);
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "dxf" => DxfWriter::new(&doc)
            .write_to_file(path)
            .map_err(|e| e.to_string()),
        _ => DwgWriter::write_to_file(path, &doc).map_err(|e| e.to_string()),
    }
}

/// Save using the document's existing version.
pub fn save(doc: &CadDocument, path: &Path) -> Result<(), String> {
    save_as_version(doc, path, doc.version)
}

// ── Post-load fixups ──────────────────────────────────────────────────────

// Resolve the current text / dimension / multiline style from the handle the
// DWG header stores into the *name* the app reads. DXF stores these as names
// directly ($TEXTSTYLE / $DIMSTYLE / $CMLSTYLE), but DWG only stores handles,
// so without this the current-style markers (and any code keyed on the name)
// fall back to "Standard". Only overrides when the handle resolves, leaving the
// DXF-provided names intact.
fn fix_current_style_names(doc: &mut CadDocument) {
    use acadrust::objects::ObjectType;

    let h = doc.header.current_text_style_handle;
    if h.is_valid() {
        if let Some(name) = doc.text_styles.iter().find(|s| s.handle == h).map(|s| s.name.clone()) {
            doc.header.current_text_style_name = name;
        }
    }
    let h = doc.header.current_dimstyle_handle;
    if h.is_valid() {
        if let Some(name) = doc.dim_styles.iter().find(|s| s.handle == h).map(|s| s.name.clone()) {
            doc.header.current_dimstyle_name = name;
        }
    }
    let h = doc.header.current_multiline_style_handle;
    if h.is_valid() {
        if let Some(name) = doc.objects.values().find_map(|o| match o {
            ObjectType::MLineStyle(s) if s.handle == h => Some(s.name.clone()),
            _ => None,
        }) {
            doc.header.multiline_style = name;
        }
    }

    // Current table / multileader style. DXF carries these as $CTABLESTYLE /
    // $CMLEADERSTYLE header vars (already read). DWG has no header field for
    // them — they live in the variable dictionary as DICTIONARYVAR entries
    // keyed "CTABLESTYLE" / "CMLEADERSTYLE". Resolve from there when present;
    // a missing entry simply leaves the existing value untouched.
    if let Some(v) = vardict_value(doc, "CTABLESTYLE") {
        if !v.is_empty() {
            doc.header.current_table_style_name = v;
        }
    }
    if let Some(v) = vardict_value(doc, "CMLEADERSTYLE") {
        if !v.is_empty() {
            doc.header.current_mleader_style_name = v;
        }
    }
}

/// Find the handle of the `DictionaryVariable` registered under `name` in any
/// of the document's dictionaries (the variable dictionary).
fn vardict_handle(doc: &CadDocument, name: &str) -> Option<acadrust::Handle> {
    use acadrust::objects::ObjectType;
    doc.objects.values().find_map(|o| {
        let entries = match o {
            ObjectType::Dictionary(d) => &d.entries,
            ObjectType::DictionaryWithDefault(d) => &d.entries,
            _ => return None,
        };
        entries
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, h)| *h)
    })
}

/// Look up a system-variable value stored in the document's variable
/// dictionary.
fn vardict_value(doc: &CadDocument, name: &str) -> Option<String> {
    use acadrust::objects::ObjectType;
    let handle = vardict_handle(doc, name)?;
    match doc.objects.get(&handle) {
        Some(ObjectType::DictionaryVariable(v)) => Some(v.value.clone()),
        _ => None,
    }
}

/// Write a value into an existing variable-dictionary entry. No-op when the
/// entry is absent (e.g. a brand-new document with no variable dictionary).
fn set_vardict_value(doc: &mut CadDocument, name: &str, value: &str) {
    use acadrust::objects::ObjectType;
    if let Some(h) = vardict_handle(doc, name) {
        if let Some(ObjectType::DictionaryVariable(v)) = doc.objects.get_mut(&h) {
            v.value = value.to_string();
        }
    }
}

/// Mirror the current table / multileader style from the header into the
/// variable dictionary before saving, so the choice round-trips through DWG
/// (which stores them as DICTIONARYVAR entries, not header fields). DXF keeps
/// both the header vars and the dictionary in sync this way too.
fn sync_current_style_vardicts(doc: &mut CadDocument) {
    let table = doc.header.current_table_style_name.clone();
    let mleader = doc.header.current_mleader_style_name.clone();
    set_vardict_value(doc, "CTABLESTYLE", &table);
    set_vardict_value(doc, "CMLEADERSTYLE", &mleader);
}

// ── Corrupt-entity guard ──────────────────────────────────────────────────
//
// acadrust's DWG parser occasionally desynchronises on certain files and
// produces entities with garbage fields: non-unit normals (components in
// 1e200+), nonsensical vertex counts (e.g. 100000), or infinite/NaN
// coordinates.  Tessellating such entities triggers huge allocations and
// numerical blow-ups in the wire pipeline.
//
// `purge_corrupt_entities` scans the document and removes any entity that
// fails a cheap sanity check, returning the number dropped so the caller can
// surface it to the UI / log.

fn finite_unit_normal(n: &acadrust::types::Vector3) -> bool {
    let (x, y, z) = (n.x, n.y, n.z);
    if !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return false;
    }
    let mag2 = x * x + y * y + z * z;
    // Accept anything within ~10% of unit length. Real files sometimes
    // store slightly denormalised normals from rounding.
    (mag2 - 1.0).abs() < 0.21
}

fn finite_coord(v: f64) -> bool {
    v.is_finite() && v.abs() < 1.0e12
}

fn finite_vec3(v: &acadrust::types::Vector3) -> bool {
    finite_coord(v.x) && finite_coord(v.y) && finite_coord(v.z)
}

/// Returns true if the entity looks like parser garbage and should be dropped.
pub(crate) fn is_entity_corrupt(e: &EntityType) -> bool {
    use acadrust::entities::EntityType as E;
    // Reject polylines at or above this vertex count. Even valid drawings
    // rarely use this many — and parser desync produces exactly-100_000-vertex
    // junk records.
    const MAX_VERTS: usize = 100_000;
    match e {
        E::LwPolyline(p) => {
            !finite_unit_normal(&p.normal)
                || p.vertices.len() >= MAX_VERTS
                || !finite_coord(p.elevation)
                || p.elevation.abs() > 1.0e10
                || !finite_coord(p.thickness)
                || p.thickness.abs() > 1.0e10
                || p.vertices
                    .iter()
                    .any(|v| !finite_coord(v.location.x) || !finite_coord(v.location.y))
        }
        E::Polyline2D(p) => {
            !finite_unit_normal(&p.normal)
                || p.vertices.len() >= MAX_VERTS
                || !finite_coord(p.elevation)
                || p.elevation.abs() > 1.0e10
                || !finite_coord(p.thickness)
                || p.thickness.abs() > 1.0e10
                || p.vertices.iter().any(|v| !finite_vec3(&v.location))
        }
        E::Polyline3D(p) => {
            p.vertices.len() >= MAX_VERTS
                || p.vertices.iter().any(|v| !finite_vec3(&v.position))
        }
        E::Polyline(p) => {
            p.vertices.len() >= MAX_VERTS
                || p.vertices.iter().any(|v| !finite_vec3(&v.location))
        }
        E::Line(l) => !finite_vec3(&l.start) || !finite_vec3(&l.end),
        E::Circle(c) => {
            !finite_vec3(&c.center)
                || !finite_coord(c.radius)
                // Reject zero- or near-zero circles: they tessellate into a
                // degenerate truck curve that crashes parameter_division.
                || c.radius.abs() < 1.0e-10
                || c.radius.abs() > 1.0e10
        }
        E::Arc(a) => {
            !finite_vec3(&a.center)
                || !finite_coord(a.radius)
                || !a.start_angle.is_finite()
                || !a.end_angle.is_finite()
                // Same degenerate-curve guard as Circle.
                || a.radius.abs() < 1.0e-10
                || a.radius.abs() > 1.0e10
                // Zero-sweep arc (start_angle == end_angle, modulo 2π) collapses
                // to a single point in WCS — truck's circle_arc on three
                // coincident vertices recurses unboundedly in parameter_division.
                || (a.end_angle - a.start_angle).abs() < 1.0e-9
                // Near-zero sweep is the same trap with a wider mouth: a tiny but
                // non-zero sweep (e.g. 1.6e-6 rad) still places start/mid/end
                // within truck's coincidence tolerance, so parameter_division
                // recurses and allocates until OOM. Gate on arc *length*
                // (radius × sweep), not sweep alone, so a legitimately large-
                // radius small-sweep arc (still a visible curve) survives while
                // sub-precision arcs are dropped.
                || a.radius.abs() * (a.end_angle - a.start_angle).abs() < 1.0e-6
                || !finite_unit_normal(&a.normal)
        }
        E::Ellipse(e) => {
            !finite_vec3(&e.center)
                || !finite_vec3(&e.major_axis)
                || !e.start_parameter.is_finite()
                || !e.end_parameter.is_finite()
                || (e.end_parameter - e.start_parameter).abs() < 1.0e-9
                || {
                    let m2 = e.major_axis.x * e.major_axis.x
                        + e.major_axis.y * e.major_axis.y
                        + e.major_axis.z * e.major_axis.z;
                    !m2.is_finite() || m2 < 1.0e-20 || m2 > 1.0e20
                }
                || !e.minor_axis_ratio.is_finite()
                || e.minor_axis_ratio.abs() < 1.0e-10
        }
        E::Spline(s) => {
            // Parser desync emits exactly-100_000-control-point splines with a
            // garbage knot vector. Building a truck NURBS/B-spline from one and
            // tessellating it runs `parameter_division` into an unbounded
            // allocation — single-threaded, 32 GB+ — long before the drawing
            // finishes loading. Reject the desync signature plus any spline
            // truck can't build: non-finite control points, or a knot vector
            // that's non-finite, non-monotonic, or the wrong length
            // (truck requires `knots.len() == ctrl.len() + degree + 1`).
            let n = s.control_points.len();
            let degree_bad = s.degree < 1;
            let deg = s.degree.max(0) as usize;
            let knots_bad = !s.knots.is_empty()
                && (s.knots.iter().any(|k| !k.is_finite())
                    || s.knots.windows(2).any(|w| w[1] < w[0])
                    || s.knots.len() != n + deg + 1);
            n >= MAX_VERTS
                || degree_bad
                || s.control_points.iter().any(|p| !finite_vec3(p))
                || knots_bad
        }
        _ => false,
    }
}

pub fn purge_corrupt_entities(doc: &mut CadDocument) -> usize {
    use rayon::prelude::*;
    // Detection is pure and read-only; the per-vertex finite/extent checks on
    // large polylines dominate, so fan the scan out across cores. Gather
    // entity references in one pass, test in parallel, then remove serially
    // (`remove_entity` needs `&mut doc`).
    let entities: Vec<&EntityType> = doc.entities().collect();
    let bad: Vec<acadrust::Handle> = entities
        .par_iter()
        .filter(|e| is_entity_corrupt(e))
        .map(|e| e.common().handle)
        .collect();
    let n = bad.len();
    for h in bad {
        doc.remove_entity(h);
    }
    n
}

/// acadrust's ViewportStatusFlags::from_bits() maps bit 0 → is_on and bit 15 → locked,
/// but the real DXF/DWG spec uses bit 15 (0x8000) → viewport on and bit 14 (0x4000) → locked.
/// Files from AutoCAD and other tools always set bit 15 for active viewports, leaving bit 0
/// clear, so acadrust reads every such viewport as off.  Correct that here after loading.
fn fix_viewport_status_flags(doc: &mut CadDocument) {
    for entity in doc.entities_mut() {
        if let EntityType::Viewport(vp) = entity {
            let bits = vp.status.to_bits();
            // If bit 0 is not set but bit 15 is, this is an external-format viewport:
            // treat bit 15 as "on" and bit 14 as "locked".
            if (bits & 0x0001) == 0 && (bits & 0x8000) != 0 {
                vp.status.is_on = true;
                vp.status.locked = (bits & 0x4000) != 0;
            }
        }
    }
}

/// The acadrust DXF reader stores several rotation fields directly from DXF
/// group code 50 in degrees, while DWG and our own creation code store radians.
/// Apply to_radians() on load so tessellation can call cos/sin uniformly.
fn fix_dxf_dimension_rotations(doc: &mut CadDocument) {
    for entity in doc.entities_mut() {
        match entity {
            EntityType::Dimension(Dimension::Linear(d)) => {
                d.rotation = d.rotation.to_radians();
            }
            EntityType::AttributeDefinition(a) => {
                a.rotation = a.rotation.to_radians();
            }
            EntityType::AttributeEntity(a) => {
                a.rotation = a.rotation.to_radians();
            }
            EntityType::Shape(s) => {
                s.rotation = s.rotation.to_radians();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod layer_roundtrip_tests {
    use super::*;
    use acadrust::tables::layer::Layer as DocLayer;

    // Add `count` new layers the way the UI does (allocate_handle, then add),
    // round-trip through `ext`, and return whether every one survived.
    fn roundtrip_layers(ext: &str, count: usize) -> bool {
        let mut doc = CadDocument::new();
        crate::linetypes::populate_document(&mut doc);
        let names: Vec<String> = (0..count).map(|n| format!("Layer{}", n + 1)).collect();
        for name in &names {
            let mut dl = DocLayer::new(name);
            dl.handle = doc.allocate_handle();
            doc.layers.add(dl).unwrap();
        }
        let path = std::env::temp_dir().join(format!("ocs_layer_rt_{count}.{ext}"));
        save_as_version(&doc, &path, acadrust::DxfVersion::AC1032).expect("save");
        let loaded = load_file(&path).expect("load");
        let _ = std::fs::remove_file(&path);
        names.iter().all(|n| loaded.layers.contains(n))
    }

    #[test]
    fn dwg_preserves_new_layer() {
        assert!(roundtrip_layers("dwg", 1), "DWG dropped the new layer (issue #67)");
    }

    #[test]
    fn dxf_preserves_new_layer() {
        assert!(roundtrip_layers("dxf", 1), "DXF dropped the new layer");
    }

    // Each new layer must get a distinct handle, or they collide and all but
    // the last are dropped on a handle-based DWG save (issue #67).
    #[test]
    fn dwg_preserves_multiple_new_layers() {
        assert!(roundtrip_layers("dwg", 3), "DWG dropped colliding new layers (issue #67)");
    }
}

#[cfg(test)]
mod corrupt_guard_tests {
    use super::*;
    use acadrust::entities::{Arc, EntityType, Spline};
    use acadrust::types::Vector3;

    // A near-zero-sweep arc: sweep 1.56e-6 rad on a 3.9e-3 radius. The angles
    // are individually finite and the radius is in range, so the old
    // (end-start) < 1e-9 check passed it through — but start/mid/end land
    // within truck's coincidence tolerance and parameter_division allocates
    // until OOM. The arc-length floor must reject it.
    #[test]
    fn rejects_near_degenerate_arc() {
        let mut a = Arc::new();
        a.center = Vector3::new(2880.84, 891.83, 0.0);
        a.radius = 0.0038974142851181423;
        a.start_angle = 1.0401656235942365;
        a.end_angle = 1.0401671831670538;
        a.normal = Vector3::new(0.0, 0.0, 1.0);
        assert!(is_entity_corrupt(&EntityType::Arc(a)));
    }

    // A large-radius small-sweep arc is still a visible curve and must survive:
    // radius 1e6 × sweep 1e-4 ≈ 100 units of arc.
    #[test]
    fn keeps_large_radius_small_sweep_arc() {
        let mut a = Arc::new();
        a.radius = 1.0e6;
        a.start_angle = 0.0;
        a.end_angle = 1.0e-4;
        a.normal = Vector3::new(0.0, 0.0, 1.0);
        assert!(!is_entity_corrupt(&EntityType::Arc(a)));
    }

    // Parser desync emits 100_000-control-point splines; building a truck
    // NURBS from one and tessellating it OOMs. The control-point cap rejects it.
    #[test]
    fn rejects_desync_spline() {
        let pts = vec![Vector3::new(0.0, 0.0, 0.0); 100_000];
        let s = Spline::from_control_points(3, pts);
        assert!(is_entity_corrupt(&EntityType::Spline(s)));
    }

    // A normal cubic spline (4 control points, valid clamped knots) survives.
    #[test]
    fn keeps_valid_spline() {
        let pts = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(2.0, -1.0, 0.0),
            Vector3::new(3.0, 0.0, 0.0),
        ];
        let s = Spline::from_control_points(3, pts);
        assert!(!is_entity_corrupt(&EntityType::Spline(s)));
    }
}
