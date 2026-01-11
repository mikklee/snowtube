// Snowflake spinner shader - samples a texture with rotation and tinting

struct Uniforms {
    size: vec2<f32>,
    time: f32,
    _padding: f32,
    color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var logo_texture: texture_2d<f32>;

@group(0) @binding(2)
var logo_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );

    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );

    var out: VertexOutput;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

// Rotate UV coordinates around center
fn rotate_uv(uv: vec2<f32>, angle: f32) -> vec2<f32> {
    let center = vec2<f32>(0.5, 0.5);
    let centered = uv - center;
    let c = cos(angle);
    let s = sin(angle);
    let rotated = vec2<f32>(
        centered.x * c - centered.y * s,
        centered.x * s + centered.y * c
    );
    return rotated + center;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Rotate based on time (clockwise)
    let rotation = -uniforms.time * 3.5;
    let rotated_uv = rotate_uv(in.uv, rotation);

    // Sample the texture
    let tex_color = textureSample(logo_texture, logo_sampler, rotated_uv);

    // Tint with theme color - multiply RGB by uniform color, keep texture alpha
    let tinted = vec4<f32>(
        uniforms.color.rgb * tex_color.a,
        tex_color.a
    );

    return tinted;
}
