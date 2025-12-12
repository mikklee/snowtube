struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct Uniforms {
    rect: vec4<f32>,
}

@group(0) @binding(0)
var tex_y: texture_2d<f32>;

@group(0) @binding(1)
var tex_uv: texture_2d<f32>;

@group(0) @binding(2)
var s: sampler;

@group(0) @binding(3)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var quad = array<vec4<f32>, 6>(
        vec4<f32>(uniforms.rect.xy, 0.0, 0.0),
        vec4<f32>(uniforms.rect.zy, 1.0, 0.0),
        vec4<f32>(uniforms.rect.xw, 0.0, 1.0),
        vec4<f32>(uniforms.rect.zy, 1.0, 0.0),
        vec4<f32>(uniforms.rect.zw, 1.0, 1.0),
        vec4<f32>(uniforms.rect.xw, 0.0, 1.0),
    );

    var out: VertexOutput;
    out.uv = quad[in_vertex_index].zw;
    out.position = vec4<f32>(quad[in_vertex_index].xy, 1.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample Y and UV planes
    let y = textureSample(tex_y, s, in.uv).r;
    let uv = textureSample(tex_uv, s, in.uv).rg;

    // NV12 limited range (16-235 for Y, 16-240 for UV) to full range
    // Y' = (Y - 16) / 219, Cb/Cr = (UV - 128) / 224
    let y_scaled = (y * 255.0 - 16.0) / 219.0;
    let cb = (uv.r * 255.0 - 128.0) / 224.0;
    let cr = (uv.g * 255.0 - 128.0) / 224.0;

    // BT.709 YCbCr to RGB conversion
    // R = Y + 1.5748 * Cr
    // G = Y - 0.1873 * Cb - 0.4681 * Cr
    // B = Y + 1.8556 * Cb
    var rgb = vec3<f32>(
        y_scaled + 1.5748 * cr,
        y_scaled - 0.1873 * cb - 0.4681 * cr,
        y_scaled + 1.8556 * cb
    );

    // Clamp to valid range
    rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));

    return vec4<f32>(rgb, 1.0);
}
