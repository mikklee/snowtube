// Audio spectrum visualizer shader
// Renders frequency bars with glow effect

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct Uniforms {
    rect: vec4<f32>,          // x1, y1, x2, y2 in clip space
    time: vec4<f32>,          // time in x, rest unused (for alignment)
}

// Spectrum data as uniform array (64 bands)
struct SpectrumData {
    bands: array<vec4<f32>, 16>,  // 64 floats packed as 16 vec4s
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<uniform> spectrum: SpectrumData;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var quad = array<vec4<f32>, 6>(
        vec4<f32>(uniforms.rect.xy, 0.0, 1.0),
        vec4<f32>(uniforms.rect.zy, 1.0, 1.0),
        vec4<f32>(uniforms.rect.xw, 0.0, 0.0),
        vec4<f32>(uniforms.rect.zy, 1.0, 1.0),
        vec4<f32>(uniforms.rect.zw, 1.0, 0.0),
        vec4<f32>(uniforms.rect.xw, 0.0, 0.0),
    );

    var out: VertexOutput;
    out.uv = quad[in_vertex_index].zw;
    out.position = vec4<f32>(quad[in_vertex_index].xy, 0.0, 1.0);
    return out;
}

// Get spectrum value at index (unpacking from vec4 array)
fn get_band(index: u32) -> f32 {
    let vec_idx = index / 4u;
    let component = index % 4u;
    let v = spectrum.bands[vec_idx];
    switch component {
        case 0u: { return v.x; }
        case 1u: { return v.y; }
        case 2u: { return v.z; }
        default: { return v.w; }
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let num_bands = 64u;

    // Which bar are we in?
    let bar_width = 1.0 / f32(num_bands);
    let bar_index = u32(uv.x / bar_width);
    let bar_x = fract(uv.x / bar_width);

    // Get the magnitude for this bar
    let magnitude = get_band(bar_index);

    let bar_gap = 0.15;
    let in_bar = bar_x > bar_gap && bar_x < (1.0 - bar_gap);
    // uv.y: 0=bottom, 1=top in vertex shader mapping
    // We want bars to grow from bottom, so fill where uv.y < magnitude
    let y_normalized = uv.y;

    if in_bar && y_normalized < magnitude {
        // Green at bottom, red at top
        let r = y_normalized;
        let g = 1.0 - y_normalized;
        return vec4<f32>(r, g, 0.0, 1.0);
    }

    // Dark background
    return vec4<f32>(0.0, 0.0, 0.0, 0.7);
}
