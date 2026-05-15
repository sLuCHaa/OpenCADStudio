// HatchModel — CPU-side hatch fill data; rendered entirely on the GPU.
use std::sync::Arc;
//
// The boundary is a closed polygon in world XY coordinates.
// The GPU fragment shader performs point-in-polygon and hatch-line tests so
// no line geometry is ever tessellated on the CPU.

pub const MAX_HATCH_BOUNDARY_VERTS: usize = 1024;

/// One line family from a PAT-format hatch pattern.
///
/// Format mirrors the standard PAT line format:
///   `angle_deg, x0, y0, dx, dy [, dash1, dash2, ...]`
///
/// The perpendicular spacing between adjacent parallel lines is:
///   `| -dx * sin(angle) + dy * cos(angle) |`
#[derive(Clone, Debug)]
pub struct PatFamily {
    /// Line direction in degrees.
    pub angle_deg: f32,
    /// Origin of the first line in this family.
    pub x0: f32,
    pub y0: f32,
    /// Step vector to the next parallel line.
    pub dx: f32,
    pub dy: f32,
    /// Dash/gap sequence: positive = dash length, negative = gap length.
    /// Empty = solid (no dash pattern).
    pub dashes: Vec<f32>,
}

/// Hatch fill pattern.
#[derive(Clone, Debug)]
pub enum HatchPattern {
    /// Opaque solid fill.
    Solid,
    /// One or more line families (PAT format).
    Pattern(Vec<PatFamily>),
    /// Linear gradient from `color` to `color2` along `angle_deg`.
    Gradient { angle_deg: f32, color2: [f32; 4] },
}

/// A hatched region defined by a closed polygon boundary.
#[derive(Clone, Debug)]
pub struct HatchModel {
    /// World XY coordinates of the boundary polygon vertices.
    /// Arc so HatchModel::clone() is a pointer bump rather than a full Vec copy.
    pub boundary: Arc<Vec<[f32; 2]>>,
    /// Fill pattern.
    pub pattern: HatchPattern,
    /// Catalog name for this pattern (e.g. "ANSI31", "SOLID", "LINEAR").
    /// Stored so `add_hatch()` can write the correct name to the DXF entity.
    pub name: String,
    /// RGBA color in [0,1].
    pub color: [f32; 4],
    /// Pattern rotation offset in radians (from DXF `pattern_angle`).
    /// Applied on top of each family's base angle at render time.
    pub angle_offset: f32,
    /// Pattern scale multiplier (from DXF `pattern_scale`).
    pub scale: f32,
    /// Optional world-space XY rect [x0, y0, x1, y1] for paper-space
    /// viewport clipping. When `Some`, the pipeline translates it into
    /// a per-frame pixel scissor and clips this hatch's draw call to it,
    /// preventing viewport-projected content from bleeding past the
    /// viewport frame. Mirrors `WireModel.vp_scissor`.
    pub vp_scissor: Option<[f32; 4]>,
}
