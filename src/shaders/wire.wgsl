// Wire shader — renders 1-D CAD entities as screen-aligned quads.
// Topology: TriangleList, 6 vertices drawn per INSTANCE.
//
// One instance = one segment. The six vertex IDs map to the corners of a
// two-triangle quad; the vertex shader derives `which_end` (0=A end, 1=B end)
// and `side` (±1 perpendicular) from `@builtin(vertex_index)` and expands the
// quad by `half_width` pixels perpendicular to the segment direction in
// screen space.
//
// Linetype is applied entirely on the GPU:
//   • distance = cumulative arc-length, linearly interpolated from
//     (distance_a, distance_b) by `which_end`.
//   • pattern_length > 0 enables the dash test; 0 = solid (no discard).
//   • pat0/pat1 encode up to 8 elements: positive=dash, negative=gap.

struct Uniforms {
    view_proj:        mat4x4<f32>,
    camera_pos:       vec4<f32>,
    viewport_size:    vec2<f32>,
    world_per_pixel:  f32,
    // LWDISPLAY toggle: 0.0 = force 1 px (half_width 0.5), 1.0 = use the
    // per-instance baked half_width. Lets the LWT button switch without
    // retessellating.
    lwdisplay_enable: f32,
    // Mesh flat-shade flag (unused here; kept so the field offsets match
    // the shared Uniforms buffer layout).
    flat_shade: f32,
    // Transparency-display toggle: 1.0 = honour baked alpha, 0.0 = force
    // every line opaque.
    transparency_enable: f32,
    _pad: vec2<f32>,
    // ── Relative-to-eye (double-single) ──────────────────────────────────
    // view_rot is the rotation-only view-projection; vertices subtract the eye
    // (eye_high + eye_low, two f32 emulating f64) before transforming, so the
    // large eye translation never enters the f32 matrix → no large-coordinate
    // jitter on pan / zoom / rotate.
    view_rot:         mat4x4<f32>,
    eye_high:         vec3<f32>,
    _pad_eh:          f32,
    eye_low:          vec3<f32>,
    _pad_el:          f32,
}
@group(0) @binding(0) var<uniform> u: Uniforms;

struct InstanceIn {
    @location(0) pos_a:          vec3<f32>,
    @location(1) pos_b:          vec3<f32>,
    @location(2) color:          vec4<f32>,
    @location(3) distance_a:     f32,
    @location(4) distance_b:     f32,
    @location(5) half_width:     f32,
    @location(6) pattern_length: f32,
    @location(7) pat0:           vec4<f32>,
    @location(8) pat1:           vec4<f32>,
    @location(9) draw_depth:     f32,
    // Double-single low residuals of the endpoints.
    @location(10) pos_a_low:     vec3<f32>,
    @location(11) pos_b_low:     vec3<f32>,
}

// Draw-order depth bias: shifts clip-space z so 2D entities of different
// types order against each other through the shared LessEqual depth test.
// draw_depth is signed (-1,1): front → positive → smaller z → drawn on top;
// 0.0 = neutral (real depth). Depth32Float gives ample precision.
const DRAW_ORDER_BIAS: f32 = 0.001;

struct VertexOut {
    @builtin(position)              clip_pos:       vec4<f32>,
    @location(0)                    color:          vec4<f32>,
    @location(1)                    distance:       f32,
    @location(2)                    pattern_length: f32,
    @location(3)                    pat0:           vec4<f32>,
    @location(4)                    pat1:           vec4<f32>,
    // World length of the smallest non-zero dash / gap element of this
    // instance. Flat-interpolated (constant per instance) so the
    // fragment stage can short-circuit the dash test when every gap
    // projects below one pixel on screen. See the LOD branch in
    // `fs_main`.
    @location(5) @interpolate(flat) min_elem:       f32,
}

@vertex fn vs_main(@builtin(vertex_index) vid: u32, in: InstanceIn) -> VertexOut {
    // Two-triangle quad corner table:
    //   vid 0,1,2 = (A,-1) (B,-1) (B,+1)
    //   vid 3,4,5 = (A,-1) (B,+1) (A,+1)
    let which_end_arr = array<f32, 6>(0.0, 1.0, 1.0, 0.0, 1.0, 0.0);
    let side_arr      = array<f32, 6>(-1.0, -1.0, 1.0, -1.0, 1.0, 1.0);
    let which_end = which_end_arr[vid];
    let side      = side_arr[vid];

    // Double-single relative-to-eye: subtract the eye from each endpoint with
    // both halves of the f64-emulating pair, then transform by the rotation-only
    // view-projection. (pos_high − eye_high) is exact in f32 for same-magnitude
    // operands (Sterbenz); adding (pos_low − eye_low) restores the residual both
    // the vertex and the eye would otherwise lose — so geometry stays put at
    // UTM-scale coordinates and after a cross-drawing paste, with no jitter.
    let rel_a = (in.pos_a - u.eye_high) + (in.pos_a_low - u.eye_low);
    let rel_b = (in.pos_b - u.eye_high) + (in.pos_b_low - u.eye_low);
    let clip_a = u.view_rot * vec4<f32>(rel_a, 1.0);
    let clip_b = u.view_rot * vec4<f32>(rel_b, 1.0);

    // NDC of both endpoints.
    let ndc_a = clip_a.xy / clip_a.w;
    let ndc_b = clip_b.xy / clip_b.w;

    // Screen-space pixel positions.
    let screen_a = ndc_a * u.viewport_size * 0.5;
    let screen_b = ndc_b * u.viewport_size * 0.5;

    // Screen-space perpendicular to segment direction.
    let seg = screen_b - screen_a;
    let seg_len = length(seg);
    var perp: vec2<f32>;
    if seg_len > 1e-4 {
        let dir = seg / seg_len;
        perp = vec2<f32>(-dir.y, dir.x);
    } else {
        perp = vec2<f32>(0.0, 1.0);
    }

    // Convert perpendicular from screen pixels to NDC offset.
    let perp_ndc = perp / (u.viewport_size * 0.5);

    // Select the clip-space position for this vertex's endpoint.
    let clip_pos = mix(clip_a, clip_b, which_end);

    // LWDISPLAY off → collapse to a 1-pixel-wide line (half_width = 0.5).
    let hw = select(0.5, in.half_width, u.lwdisplay_enable > 0.5);

    // Offset in clip space (multiply by w to un-apply perspective division).
    let ndc_offset = perp_ndc * hw * side;
    let final_clip = clip_pos + vec4<f32>(ndc_offset * clip_pos.w, 0.0, 0.0);

    // Smallest non-zero dash / gap element, in world units. Used by
    // the fragment stage to decide when the pattern's finest feature
    // would render below one pixel and should collapse to a solid line.
    var min_elem: f32 = in.pattern_length;
    let elems = array<f32, 8>(
        in.pat0.x, in.pat0.y, in.pat0.z, in.pat0.w,
        in.pat1.x, in.pat1.y, in.pat1.z, in.pat1.w,
    );
    for (var i = 0u; i < 8u; i++) {
        let e = abs(elems[i]);
        if e > 0.0 && e < min_elem { min_elem = e; }
    }

    var out: VertexOut;
    out.clip_pos       = final_clip;
    out.clip_pos.z     = out.clip_pos.z - in.draw_depth * DRAW_ORDER_BIAS * out.clip_pos.w;
    out.color          = in.color;
    out.distance       = mix(in.distance_a, in.distance_b, which_end);
    out.pattern_length = in.pattern_length;
    out.pat0           = in.pat0;
    out.pat1           = in.pat1;
    out.min_elem       = min_elem;
    return out;
}

// Returns true if arc-length `dist` falls inside a dash segment.
fn in_dash(dist: f32, pat_len: f32, p0: vec4<f32>, p1: vec4<f32>) -> bool {
    let d   = ((dist % pat_len) + pat_len) % pat_len;
    var pos = 0.0f;
    let elems = array<f32, 8>(p0.x, p0.y, p0.z, p0.w, p1.x, p1.y, p1.z, p1.w);
    for (var i = 0u; i < 8u; i++) {
        let elem = elems[i];
        if elem == 0.0 { break; }
        let len = abs(elem);
        if d < pos + len { return elem > 0.0; }
        pos += len;
    }
    return false;
}

@fragment fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    if in.pattern_length > 0.0 {
        // LOD: once the pattern's smallest feature drops below ~1 px
        // on screen, dash gaps alias / shimmer (or vanish completely)
        // and the user reads the line as solid anyway. Skip the dash
        // test and return solid colour — also saves the per-fragment
        // arc-length math + `discard`.
        if in.min_elem >= u.world_per_pixel {
            if !in_dash(in.distance, in.pattern_length, in.pat0, in.pat1) {
                discard;
            }
        }
    }
    // Transparency display off → force the line opaque.
    let alpha = select(1.0, in.color.a, u.transparency_enable > 0.5);
    return vec4<f32>(in.color.rgb, alpha);
}
