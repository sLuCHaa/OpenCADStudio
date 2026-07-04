use acadrust::entities::{mesh::Mesh, polygon_mesh::PolygonMesh, Face3D, PolyfaceMesh};
use glam::Vec3;

use crate::command::EntityTransform;
use crate::entities::common::{ro_prop as ro, square_grip};
use crate::entities::traits::{Grippable, PropertyEditable, Transformable, TruckConvertible};
use crate::scene::convert::acad_to_truck::{TruckEntity, TruckObject};
use crate::scene::model::object::{GripApply, GripDef, PropSection};
use crate::scene::model::wire_model::SnapHint;

/// Triangulate a planar (possibly concave) polygon into a flat triangle-soup
/// (3 vertices per triangle), preserving the polygon's winding. A simple fan
/// from vertex 0 is only valid for convex faces — a concave face (e.g. an
/// L-shaped mesh face) fans into triangles that spill outside the outline. Ear
/// clipping handles both. Falls back to a fan when the polygon is degenerate.
fn triangulate_planar(poly: &[[f64; 3]]) -> Vec<[f64; 3]> {
    let n = poly.len();
    if n < 3 {
        return Vec::new();
    }
    if n == 3 {
        return vec![poly[0], poly[1], poly[2]];
    }
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let fan = || {
        let mut out = Vec::new();
        for i in 1..n - 1 {
            out.push(poly[0]);
            out.push(poly[i]);
            out.push(poly[i + 1]);
        }
        out
    };
    // Face normal via Newell's method (robust for near-planar polygons).
    let mut normal = [0.0f64; 3];
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        normal[0] += (a[1] - b[1]) * (a[2] + b[2]);
        normal[1] += (a[2] - b[2]) * (a[0] + b[0]);
        normal[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }
    let nlen = dot(normal, normal).sqrt();
    if nlen < 1e-12 {
        return fan();
    }
    let normal = [normal[0] / nlen, normal[1] / nlen, normal[2] / nlen];
    // Orthonormal in-plane basis.
    let seed = if normal[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let mut u = cross(seed, normal);
    let ul = dot(u, u).sqrt();
    if ul < 1e-12 {
        return fan();
    }
    u = [u[0] / ul, u[1] / ul, u[2] / ul];
    let v = cross(normal, u);
    let p2: Vec<[f64; 2]> = poly.iter().map(|&p| [dot(p, u), dot(p, v)]).collect();
    // Signed area → winding (CCW when positive in the (u, v) frame).
    let mut area = 0.0;
    for i in 0..n {
        let a = p2[i];
        let b = p2[(i + 1) % n];
        area += a[0] * b[1] - b[0] * a[1];
    }
    let ccw = area > 0.0;
    let tri_area2 = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    let in_tri = |p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        let d1 = tri_area2(a, b, p);
        let d2 = tri_area2(b, c, p);
        let d3 = tri_area2(c, a, p);
        let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
        let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
        !(neg && pos)
    };
    let mut idx: Vec<usize> = (0..n).collect();
    let mut out: Vec<[f64; 3]> = Vec::with_capacity((n - 2) * 3);
    let mut guard = 0usize;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for k in 0..m {
            let i0 = idx[(k + m - 1) % m];
            let i1 = idx[k];
            let i2 = idx[(k + 1) % m];
            let (a, b, c) = (p2[i0], p2[i1], p2[i2]);
            let convex = if ccw { tri_area2(a, b, c) > 0.0 } else { tri_area2(a, b, c) < 0.0 };
            if !convex {
                continue;
            }
            let mut contains = false;
            for &j in &idx {
                if j == i0 || j == i1 || j == i2 {
                    continue;
                }
                if in_tri(p2[j], a, b, c) {
                    contains = true;
                    break;
                }
            }
            if contains {
                continue;
            }
            out.push(poly[i0]);
            out.push(poly[i1]);
            out.push(poly[i2]);
            idx.remove(k);
            clipped = true;
            break;
        }
        if !clipped {
            // No ear found (self-intersecting / numerically degenerate) — bail
            // to a fan of the remainder rather than loop forever.
            for i in 1..idx.len() - 1 {
                out.push(poly[idx[0]]);
                out.push(poly[idx[i]]);
                out.push(poly[idx[i + 1]]);
            }
            return out;
        }
    }
    if idx.len() == 3 {
        out.push(poly[idx[0]]);
        out.push(poly[idx[1]]);
        out.push(poly[idx[2]]);
    }
    out
}

// ── Face3D ────────────────────────────────────────────────────────────────────

fn v3(v: &acadrust::types::Vector3) -> [f64; 3] {
    [v.x, v.y, v.z]
}

fn dvec3(v: &acadrust::types::Vector3) -> glam::DVec3 {
    glam::DVec3::new(v.x, v.y, v.z)
}

fn v3f32(v: &acadrust::types::Vector3) -> [f32; 3] {
    [v.x as f32, v.y as f32, v.z as f32]
}

impl TruckConvertible for Face3D {
    fn to_truck(&self, _document: &acadrust::CadDocument) -> Option<TruckEntity> {
        let p0 = v3(&self.first_corner);
        let p1 = v3(&self.second_corner);
        let p2 = v3(&self.third_corner);
        let p3 = v3(&self.fourth_corner);
        let p0f = v3f32(&self.first_corner);
        let p1f = v3f32(&self.second_corner);
        let p2f = v3f32(&self.third_corner);
        let p3f = v3f32(&self.fourth_corner);
        let inv = self.invisible_edges;

        // Add edge as a line segment (separated by NaN from previous edges).
        let mut pts: Vec<[f64; 3]> = Vec::new();
        let mut add_edge = |a: [f64; 3], b: [f64; 3]| {
            if !pts.is_empty() {
                pts.push([f64::NAN; 3]);
            }
            pts.push(a);
            pts.push(b);
        };

        if !inv.is_first_invisible() {
            add_edge(p0, p1);
        }
        if !inv.is_second_invisible() {
            add_edge(p1, p2);
        }
        if !inv.is_third_invisible() {
            add_edge(p2, p3);
        }
        if !inv.is_fourth_invisible() {
            add_edge(p3, p0);
        }

        if pts.is_empty() {
            // All edges invisible — show a tiny cross at centroid.
            let cx = (p0[0] + p1[0] + p2[0] + p3[0]) / 4.0;
            let cy = (p0[1] + p1[1] + p2[1] + p3[1]) / 4.0;
            let cz = (p0[2] + p1[2] + p2[2] + p3[2]) / 4.0;
            let s = 0.1_f64;
            pts = vec![[cx - s, cy, cz], [cx + s, cy, cz]];
        }

        Some(TruckEntity {
            object: TruckObject::Lines(pts),
            snap_pts: vec![
                (Vec3::from(p0f).as_dvec3(), SnapHint::Node),
                (Vec3::from(p1f).as_dvec3(), SnapHint::Node),
                (Vec3::from(p2f).as_dvec3(), SnapHint::Node),
                (Vec3::from(p3f).as_dvec3(), SnapHint::Node),
            ],
            tangent_geoms: vec![],
            key_vertices: vec![p0, p1, p2, p3],
            fill_tris: vec![],
        })
    }
}

impl Grippable for Face3D {
    fn grips(&self) -> Vec<GripDef> {
        vec![
            square_grip(0, dvec3(&self.first_corner)),
            square_grip(1, dvec3(&self.second_corner)),
            square_grip(2, dvec3(&self.third_corner)),
            square_grip(3, dvec3(&self.fourth_corner)),
        ]
    }

    fn apply_grip(&mut self, grip_id: usize, apply: GripApply) {
        let corner = match grip_id {
            0 => &mut self.first_corner,
            1 => &mut self.second_corner,
            2 => &mut self.third_corner,
            3 => &mut self.fourth_corner,
            _ => return,
        };
        match apply {
            GripApply::Translate(d) => {
                corner.x += d.x as f64;
                corner.y += d.y as f64;
                corner.z += d.z as f64;
            }
            GripApply::Absolute(p) => {
                corner.x = p.x as f64;
                corner.y = p.y as f64;
                corner.z = p.z as f64;
            }
        }
    }
}

impl PropertyEditable for Face3D {
    fn geometry_properties(&self, _text_style_names: &[String]) -> Vec<PropSection> {
        use crate::entities::common::edit_prop as edit;
        let inv = self.invisible_edges;
        let edge = |hidden: bool| if hidden { "Invisible" } else { "Visible" };
        vec![PropSection {
            title: "Geometry".into(),
            props: vec![
                ro("Current vertex", "f3_current", String::new()),
                edit("Vertex 1 X", "f3_p1x", self.first_corner.x),
                edit("Vertex 1 Y", "f3_p1y", self.first_corner.y),
                edit("Vertex 1 Z", "f3_p1z", self.first_corner.z),
                edit("Vertex 2 X", "f3_p2x", self.second_corner.x),
                edit("Vertex 2 Y", "f3_p2y", self.second_corner.y),
                edit("Vertex 2 Z", "f3_p2z", self.second_corner.z),
                edit("Vertex 3 X", "f3_p3x", self.third_corner.x),
                edit("Vertex 3 Y", "f3_p3y", self.third_corner.y),
                edit("Vertex 3 Z", "f3_p3z", self.third_corner.z),
                edit("Vertex 4 X", "f3_p4x", self.fourth_corner.x),
                edit("Vertex 4 Y", "f3_p4y", self.fourth_corner.y),
                edit("Vertex 4 Z", "f3_p4z", self.fourth_corner.z),
                ro("Edge 1", "f3_edge1", edge(inv.is_first_invisible())),
                ro("Edge 2", "f3_edge2", edge(inv.is_second_invisible())),
                ro("Edge 3", "f3_edge3", edge(inv.is_third_invisible())),
                ro("Edge 4", "f3_edge4", edge(inv.is_fourth_invisible())),
            ],
        }]
    }

    fn apply_geom_prop(&mut self, field: &str, value: &str) {
        let Ok(v) = value.trim().parse::<f64>() else {
            return;
        };
        match field {
            "f3_p1x" => self.first_corner.x = v,
            "f3_p1y" => self.first_corner.y = v,
            "f3_p1z" => self.first_corner.z = v,
            "f3_p2x" => self.second_corner.x = v,
            "f3_p2y" => self.second_corner.y = v,
            "f3_p2z" => self.second_corner.z = v,
            "f3_p3x" => self.third_corner.x = v,
            "f3_p3y" => self.third_corner.y = v,
            "f3_p3z" => self.third_corner.z = v,
            "f3_p4x" => self.fourth_corner.x = v,
            "f3_p4y" => self.fourth_corner.y = v,
            "f3_p4z" => self.fourth_corner.z = v,
            _ => {}
        }
    }
}

impl Transformable for Face3D {
    fn apply_transform(&mut self, t: &EntityTransform) {
        crate::scene::view::transform::apply_standard_entity_transform(self, t, |entity, p1, p2| {
            for corner in [
                &mut entity.first_corner,
                &mut entity.second_corner,
                &mut entity.third_corner,
                &mut entity.fourth_corner,
            ] {
                crate::scene::view::transform::reflect_xy_point(&mut corner.x, &mut corner.y, p1, p2);
            }
        });
    }
}

// ── PolygonMesh (N×M grid) ────────────────────────────────────────────────────

impl TruckConvertible for PolygonMesh {
    fn to_truck(&self, _document: &acadrust::CadDocument) -> Option<TruckEntity> {
        let m = self.m_vertex_count as usize;
        let n = self.n_vertex_count as usize;
        if m == 0 || n == 0 || self.vertices.len() < m * n {
            return None;
        }

        let closed_m = self
            .flags
            .contains(acadrust::entities::PolygonMeshFlags::CLOSED_M);
        let closed_n = self
            .flags
            .contains(acadrust::entities::PolygonMeshFlags::CLOSED_N);

        let pt = |i: usize, j: usize| -> [f64; 3] {
            let v = &self.vertices[i * n + j];
            [v.location.x, v.location.y, v.location.z]
        };

        let mut pts: Vec<[f64; 3]> = Vec::new();
        let mut fill_tris: Vec<[f64; 3]> = Vec::new();

        // Rows (M direction).
        for i in 0..m {
            pts.push([f64::NAN; 3]);
            for j in 0..n {
                pts.push(pt(i, j));
            }
            if closed_n {
                pts.push(pt(i, 0));
            }
        }

        // Columns (N direction).
        for j in 0..n {
            pts.push([f64::NAN; 3]);
            for i in 0..m {
                pts.push(pt(i, j));
            }
            if closed_m {
                pts.push(pt(0, j));
            }
        }

        // Fill: triangulate each grid quad (two triangles per cell).
        let mi = if closed_m { m } else { m - 1 };
        let ni = if closed_n { n } else { n - 1 };
        for i in 0..mi {
            for j in 0..ni {
                let p00 = pt(i, j);
                let p10 = pt((i + 1) % m, j);
                let p01 = pt(i, (j + 1) % n);
                let p11 = pt((i + 1) % m, (j + 1) % n);
                fill_tris.extend_from_slice(&[p00, p10, p11, p00, p11, p01]);
            }
        }

        Some(TruckEntity {
            object: TruckObject::Lines(pts),
            snap_pts: vec![],
            tangent_geoms: vec![],
            key_vertices: vec![],
            fill_tris,
        })
    }
}

impl Grippable for PolygonMesh {
    fn grips(&self) -> Vec<GripDef> {
        self.vertices
            .iter()
            .enumerate()
            .map(|(i, v)| {
                square_grip(
                    i,
                    glam::DVec3::new(v.location.x, v.location.y, v.location.z),
                )
            })
            .collect()
    }

    fn apply_grip(&mut self, grip_id: usize, apply: GripApply) {
        if let Some(v) = self.vertices.get_mut(grip_id) {
            match apply {
                GripApply::Translate(d) => {
                    v.location.x += d.x as f64;
                    v.location.y += d.y as f64;
                    v.location.z += d.z as f64;
                }
                GripApply::Absolute(p) => {
                    v.location.x = p.x as f64;
                    v.location.y = p.y as f64;
                    v.location.z = p.z as f64;
                }
            }
        }
    }
}

impl PropertyEditable for PolygonMesh {
    fn geometry_properties(&self, _text_style_names: &[String]) -> Vec<PropSection> {
        let smooth = match self.smooth_type {
            acadrust::entities::polygon_mesh::SurfaceSmoothType::NoSmooth => "None",
            acadrust::entities::polygon_mesh::SurfaceSmoothType::Quadratic => "Quadratic",
            acadrust::entities::polygon_mesh::SurfaceSmoothType::Cubic => "Cubic",
            acadrust::entities::polygon_mesh::SurfaceSmoothType::Bezier => "Bezier",
        };
        let yesno = |b: bool| if b { "Yes" } else { "No" };
        let first = self.vertices.first();
        // Grid faces: one quad per cell; closed direction adds a wrap row/column.
        let m = self.m_vertex_count.max(0) as i64;
        let n = self.n_vertex_count.max(0) as i64;
        let cells_m = if self.is_closed_m() { m } else { (m - 1).max(0) };
        let cells_n = if self.is_closed_n() { n } else { (n - 1).max(0) };
        let face_count = cells_m * cells_n;
        vec![
            PropSection {
                title: "Geometry".into(),
                props: vec![
                    ro("Vertex", "pm_vertex", String::new()),
                    ro(
                        "Vertex X",
                        "pm_vx",
                        first.map(|v| format!("{:.4}", v.location.x)).unwrap_or_default(),
                    ),
                    ro(
                        "Vertex Y",
                        "pm_vy",
                        first.map(|v| format!("{:.4}", v.location.y)).unwrap_or_default(),
                    ),
                    ro(
                        "Vertex Z",
                        "pm_vz",
                        first.map(|v| format!("{:.4}", v.location.z)).unwrap_or_default(),
                    ),
                    ro("M vertex count", "pm_m", self.m_vertex_count.to_string()),
                    ro("N vertex count", "pm_n", self.n_vertex_count.to_string()),
                    ro("M closed", "pm_closed_m", yesno(self.is_closed_m())),
                    ro("N closed", "pm_closed_n", yesno(self.is_closed_n())),
                    ro("M density", "pm_smooth_m", self.m_smooth_density.to_string()),
                    ro("N density", "pm_smooth_n", self.n_smooth_density.to_string()),
                    ro("Vertex count", "pm_v", self.vertices.len().to_string()),
                    ro("Face count", "pm_faces", face_count.to_string()),
                ],
            },
            PropSection {
                title: "Misc".into(),
                props: vec![ro("Fit/smooth", "pm_smooth", smooth)],
            },
        ]
    }

    fn apply_geom_prop(&mut self, _field: &str, _value: &str) {}
}

impl Transformable for PolygonMesh {
    fn apply_transform(&mut self, t: &EntityTransform) {
        crate::scene::view::transform::apply_standard_entity_transform(self, t, |entity, p1, p2| {
            for v in &mut entity.vertices {
                crate::scene::view::transform::reflect_xy_point(
                    &mut v.location.x,
                    &mut v.location.y,
                    p1,
                    p2,
                );
            }
        });
    }
}

// ── PolyfaceMesh (arbitrary faces with 1-based vertex indices) ────────────────

impl TruckConvertible for PolyfaceMesh {
    fn to_truck(&self, _document: &acadrust::CadDocument) -> Option<TruckEntity> {
        if self.vertices.is_empty() || self.faces.is_empty() {
            return None;
        }

        let get_v = |idx: i16| -> Option<[f64; 3]> {
            let i = (idx.abs() as usize).checked_sub(1)?;
            let v = self.vertices.get(i)?;
            Some([v.location.x, v.location.y, v.location.z])
        };

        let mut pts: Vec<[f64; 3]> = Vec::new();
        let mut fill_tris: Vec<[f64; 3]> = Vec::new();

        for face in &self.faces {
            // Indices: 0 means unused. Negative = invisible edge (still render for wireframe).
            let indices = [face.index1, face.index2, face.index3, face.index4];
            let verts: Vec<[f64; 3]> = indices
                .iter()
                .filter(|&&i| i != 0)
                .filter_map(|&i| get_v(i))
                .collect();

            if verts.len() < 2 {
                continue;
            }
            pts.push([f64::NAN; 3]);
            for &p in &verts {
                pts.push(p);
            }
            // Close the face polygon.
            pts.push(verts[0]);

            // Ear-clip the face for solid fill (handles concave faces).
            if verts.len() >= 3 {
                fill_tris.extend(triangulate_planar(&verts));
            }
        }

        Some(TruckEntity {
            object: TruckObject::Lines(pts),
            snap_pts: vec![],
            tangent_geoms: vec![],
            key_vertices: vec![],
            fill_tris,
        })
    }
}

impl Grippable for PolyfaceMesh {
    fn grips(&self) -> Vec<GripDef> {
        self.vertices
            .iter()
            .enumerate()
            .map(|(i, v)| {
                square_grip(
                    i,
                    glam::DVec3::new(v.location.x, v.location.y, v.location.z),
                )
            })
            .collect()
    }

    fn apply_grip(&mut self, grip_id: usize, apply: GripApply) {
        if let Some(v) = self.vertices.get_mut(grip_id) {
            match apply {
                GripApply::Translate(d) => {
                    v.location.x += d.x as f64;
                    v.location.y += d.y as f64;
                    v.location.z += d.z as f64;
                }
                GripApply::Absolute(p) => {
                    v.location.x = p.x as f64;
                    v.location.y = p.y as f64;
                    v.location.z = p.z as f64;
                }
            }
        }
    }
}

impl PropertyEditable for PolyfaceMesh {
    fn geometry_properties(&self, _text_style_names: &[String]) -> Vec<PropSection> {
        let smooth = match self.smooth_surface {
            acadrust::entities::PolyfaceSmoothType::None => "None",
            acadrust::entities::PolyfaceSmoothType::Quadratic => "Quadratic",
            acadrust::entities::PolyfaceSmoothType::Cubic => "Cubic",
            acadrust::entities::PolyfaceSmoothType::Bezier => "Bezier",
        };
        let first = self.vertices.first();
        vec![
            PropSection {
                title: "Geometry".into(),
                props: vec![
                    ro("Vertex", "pfm_vertex", String::new()),
                    ro(
                        "Vertex X",
                        "pfm_vx",
                        first.map(|v| format!("{:.4}", v.location.x)).unwrap_or_default(),
                    ),
                    ro(
                        "Vertex Y",
                        "pfm_vy",
                        first.map(|v| format!("{:.4}", v.location.y)).unwrap_or_default(),
                    ),
                    ro(
                        "Vertex Z",
                        "pfm_vz",
                        first.map(|v| format!("{:.4}", v.location.z)).unwrap_or_default(),
                    ),
                    // Polyface meshes store an explicit vertex/face list rather
                    // than an M×N grid, so the grid-only rows are not applicable.
                    ro("M vertex count", "pfm_m", String::new()),
                    ro("N vertex count", "pfm_n", String::new()),
                    ro("M closed", "pfm_closed_m", String::new()),
                    ro("N closed", "pfm_closed_n", String::new()),
                    ro("M density", "pfm_density_m", String::new()),
                    ro("N density", "pfm_density_n", String::new()),
                    ro("Vertex count", "pfm_v", self.vertices.len().to_string()),
                    ro("Face count", "pfm_f", self.faces.len().to_string()),
                ],
            },
            PropSection {
                title: "Misc".into(),
                props: vec![ro("Fit/smooth", "pfm_smooth", smooth)],
            },
        ]
    }

    fn apply_geom_prop(&mut self, _field: &str, _value: &str) {}
}

impl Transformable for PolyfaceMesh {
    fn apply_transform(&mut self, t: &EntityTransform) {
        crate::scene::view::transform::apply_standard_entity_transform(self, t, |entity, p1, p2| {
            for v in &mut entity.vertices {
                crate::scene::view::transform::reflect_xy_point(
                    &mut v.location.x,
                    &mut v.location.y,
                    p1,
                    p2,
                );
            }
        });
    }
}

// ── Mesh (SubD mesh) ──────────────────────────────────────────────────────────
//
// Modern subdivision mesh — distinct from PolygonMesh. The render path emits
// the file's per-edge wireframe and triangulates each face into fill_tris so
// solid views still draw a shaded surface. Subdivision-level smoothing is
// honoured only as metadata; we don't run a Catmull-Clark refinement pass
// here yet.

impl TruckConvertible for Mesh {
    fn to_truck(&self, _document: &acadrust::CadDocument) -> Option<TruckEntity> {
        if self.vertices.is_empty() {
            return None;
        }
        let get = |i: usize| -> Option<[f64; 3]> { self.vertices.get(i).map(|v| [v.x, v.y, v.z]) };

        let mut pts: Vec<[f64; 3]> = Vec::new();
        if !self.edges.is_empty() {
            for edge in &self.edges {
                if let (Some(a), Some(b)) = (get(edge.start), get(edge.end)) {
                    pts.push([f64::NAN; 3]);
                    pts.push(a);
                    pts.push(b);
                }
            }
        } else {
            for face in &self.faces {
                if face.vertices.len() < 2 {
                    continue;
                }
                pts.push([f64::NAN; 3]);
                for &vi in &face.vertices {
                    if let Some(p) = get(vi) {
                        pts.push(p);
                    }
                }
                if let Some(first) = face.vertices.first().and_then(|&i| get(i)) {
                    pts.push(first);
                }
            }
        }

        // Ear-clip each face into fill_tris so shaded views render the mesh as
        // a solid surface (fan triangulation spills outside concave faces).
        let mut fill_tris: Vec<[f64; 3]> = Vec::new();
        for face in &self.faces {
            let verts: Vec<[f64; 3]> = face.vertices.iter().filter_map(|&vi| get(vi)).collect();
            if verts.len() >= 3 {
                fill_tris.extend(triangulate_planar(&verts));
            }
        }

        let snap_pts: Vec<(glam::DVec3, SnapHint)> = self
            .vertices
            .iter()
            .map(|v| (glam::DVec3::new(v.x, v.y, v.z), SnapHint::Node))
            .collect();
        let key_vertices: Vec<[f64; 3]> = self.vertices.iter().map(|v| [v.x, v.y, v.z]).collect();

        Some(TruckEntity {
            object: TruckObject::Lines(pts),
            snap_pts,
            tangent_geoms: vec![],
            key_vertices,
            fill_tris,
        })
    }
}

impl Grippable for Mesh {
    fn grips(&self) -> Vec<GripDef> {
        self.vertices
            .iter()
            .enumerate()
            .map(|(i, v)| square_grip(i, glam::DVec3::new(v.x, v.y, v.z)))
            .collect()
    }

    fn apply_grip(&mut self, grip_id: usize, apply: GripApply) {
        if let Some(v) = self.vertices.get_mut(grip_id) {
            match apply {
                GripApply::Translate(d) => {
                    v.x += d.x as f64;
                    v.y += d.y as f64;
                    v.z += d.z as f64;
                }
                GripApply::Absolute(p) => {
                    v.x = p.x as f64;
                    v.y = p.y as f64;
                    v.z = p.z as f64;
                }
            }
        }
    }
}

impl PropertyEditable for Mesh {
    fn geometry_properties(&self, _text_style_names: &[String]) -> Vec<PropSection> {
        // Watertight when every face edge is shared by exactly two faces
        // (closed manifold). Empty meshes are not watertight.
        let mut edge_use: std::collections::HashMap<(usize, usize), u32> =
            std::collections::HashMap::new();
        for face in &self.faces {
            let vs = &face.vertices;
            for i in 0..vs.len() {
                let a = vs[i];
                let b = vs[(i + 1) % vs.len()];
                let key = if a < b { (a, b) } else { (b, a) };
                *edge_use.entry(key).or_insert(0) += 1;
            }
        }
        let watertight =
            !self.faces.is_empty() && edge_use.values().all(|&c| c == 2);
        vec![PropSection {
            title: "Geometry".into(),
            props: vec![
                ro(
                    "Level of Smoothness",
                    "msh_subdiv",
                    self.subdivision_level.to_string(),
                ),
                ro("Number of Faces", "msh_f", self.faces.len().to_string()),
                ro("Number of Grips", "msh_grips", self.vertices.len().to_string()),
                ro(
                    "Watertight",
                    "msh_watertight",
                    if watertight { "Yes" } else { "No" },
                ),
            ],
        }]
    }

    fn apply_geom_prop(&mut self, _field: &str, _value: &str) {}
}

impl Transformable for Mesh {
    fn apply_transform(&mut self, t: &EntityTransform) {
        crate::scene::view::transform::apply_standard_entity_transform(self, t, |entity, p1, p2| {
            for v in &mut entity.vertices {
                crate::scene::view::transform::reflect_xy_point(&mut v.x, &mut v.y, p1, p2);
            }
        });
    }
}

#[cfg(test)]
mod triangulate_tests {
    use super::triangulate_planar;

    fn poly_area2d(p: &[[f64; 2]]) -> f64 {
        let n = p.len();
        let mut a = 0.0;
        for i in 0..n {
            let b = p[(i + 1) % n];
            a += p[i][0] * b[1] - b[0] * p[i][1];
        }
        (a * 0.5).abs()
    }
    fn tri_area(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
        // area in XY plane (test polygons are in Z=0)
        (((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) * 0.5).abs()
    }

    #[test]
    fn concave_l_shape_no_spillover() {
        // L-shaped hexagon (concave at the inner corner).
        let poly: Vec<[f64; 3]> = vec![
            [0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [4.0, 2.0, 0.0],
            [2.0, 2.0, 0.0], [2.0, 4.0, 0.0], [0.0, 4.0, 0.0],
        ];
        let p2: Vec<[f64; 2]> = poly.iter().map(|p| [p[0], p[1]]).collect();
        let want = poly_area2d(&p2); // = 12.0
        let tris = triangulate_planar(&poly);
        assert_eq!(tris.len() % 3, 0);
        assert_eq!(tris.len() / 3, poly.len() - 2, "expected n-2 triangles");
        let got: f64 = tris.chunks(3).map(|t| tri_area(t[0], t[1], t[2])).sum();
        eprintln!("concave L: want_area={want} got_area={got} tris={}", tris.len() / 3);
        // Fan would overshoot (triangles outside the L). Ear clip = exact.
        assert!((got - want).abs() < 1e-6, "triangle area {got} != polygon area {want} (spillover)");
    }
}
