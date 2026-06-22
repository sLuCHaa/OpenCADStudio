// Face3D shader — flat-shaded triangle fill for DXF 3DFACE entities.
//
// Vertex layout (28 bytes):
//   position  [f32; 3]   offset  0   12 B
//   color     [f32; 4]   offset 12   16 B

struct Uniforms {
    view_proj:           mat4x4<f32>,
    camera_pos:          vec4<f32>,
    viewport_size:       vec2<f32>,
    world_per_pixel:     f32,
    lwdisplay_enable:    f32,
    flat_shade:          f32,
    transparency_enable: f32,
    _pad:                vec2<f32>,
    // Relative-to-eye (double-single): see wire.wgsl.
    view_rot:            mat4x4<f32>,
    eye_high:            vec3<f32>,
    _pad_eh:             f32,
    eye_low:             vec3<f32>,
    _pad_el:             f32,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexIn {
    @location(0) position:   vec3<f32>,
    @location(1) color:      vec4<f32>,
    @location(2) draw_depth: f32,
};

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)       color:    vec4<f32>,
};

// Draw-order depth bias (see wire.wgsl). Signed draw_depth; 0.0 (3D mesh
// faces) leaves real depth untouched; 2D fills / 3DFACE quads order by rank.
const DRAW_ORDER_BIAS: f32 = 0.001;

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip_pos = u.view_rot * vec4<f32>((v.position - u.eye_high) - u.eye_low, 1.0);
    out.clip_pos.z = out.clip_pos.z - v.draw_depth * DRAW_ORDER_BIAS * out.clip_pos.w;
    out.color    = v.color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
