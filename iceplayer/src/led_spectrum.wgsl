// 2D LED Spectrum - Visualiser
//
// Based on work by:
// - simesgreen (https://www.shadertoy.com/view/Msl3zr) - Original Led Spectrum Analyser (2013)
// - uNiversal (https://www.shadertoy.com/view/WdlBDX) - 2D LED Spectrum (2015)
//
// Licensed under Creative Commons Attribution-NonCommercial-ShareAlike 3.0 Unported License
// https://creativecommons.org/licenses/by-nc-sa/3.0/

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct Uniforms {
    rect: vec4<f32>,
    time: vec4<f32>,
    color: vec4<f32>,
    resolution: vec4<f32>,
}

struct SpectrumData {
    bands: array<vec4<f32>, 16>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<uniform> spectrum: SpectrumData;

@group(0) @binding(2)
var noise_texture: texture_2d<f32>;

@group(0) @binding(3)
var noise_sampler: sampler;

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

// Gradient colors (purple to pink)
const DST_COLOR: vec3<f32> = vec3<f32>(216.0 / 255.0, 78.0 / 255.0, 255.0 / 255.0);
const SRC_COLOR: vec3<f32> = vec3<f32>(115.0 / 255.0, 78.0 / 255.0, 255.0 / 255.0);
const CENTER_X: f32 = 0.5;

fn get_gradient_color(uv: vec2<f32>) -> vec3<f32> {
    var m = 0.0;
    if uv.x > CENTER_X {
        m = 1.0;
    }
    let d = (uv.x - m * CENTER_X) / CENTER_X;

    var src1 = SRC_COLOR;
    var dst1 = DST_COLOR;
    if uv.x > CENTER_X {
        src1 = DST_COLOR;
        dst1 = SRC_COLOR;
    }

    let r = d * dst1.r + (1.0 - d) * src1.r;
    let g = d * dst1.g + (1.0 - d) * src1.g;
    let b = d * dst1.b + (1.0 - d) * src1.b;

    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;

    // Number of LED bands and segments
    let bands = 30.0;
    let segs = 40.0;

    // Quantize coordinates to create LED grid
    var p: vec2<f32>;
    p.x = floor(uv.x * bands) / bands;
    p.y = floor(uv.y * segs) / segs;

    // Read frequency data from spectrum
    // Map the quantized x position to our 64 spectrum bands
    let band_index = u32(p.x * 64.0);
    var fft = get_band(band_index);

    // Apply power curve for better visual response
    fft = sqrt(fft * fft * fft);

    // Boost the signal for visibility
    fft = fft * 1.5;

    // Get gradient color based on position
    let color = get_gradient_color(uv);

    // Mask for bar graph - show LED if below the FFT level
    let mask = select(0.0, 1.0, p.y < fft);

    // LED shape - create rounded rectangle effect
    let d = fract((uv - p) * vec2<f32>(bands, segs)) - 0.5;
    let led = smoothstep(0.5, 0.35, abs(d.x)) * smoothstep(0.5, 0.35, abs(d.y));

    // Final LED color
    let led_color = led * color * mask;

    // Add subtle glow effect
    let glow = led * mask * 0.3;

    return vec4<f32>(led_color + glow * color * 0.5, led * mask);
}
