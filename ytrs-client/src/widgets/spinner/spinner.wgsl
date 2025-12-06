// Spinner shader - draws a simple rotating arc

struct Uniforms {
    size: vec2<f32>,
    time: f32,
    _padding: f32,
    track_color: vec4<f32>,
    bar_color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

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

const PI: f32 = 3.14159265359;
const TWO_PI: f32 = 6.28318530718;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let center = vec2<f32>(0.5, 0.5);
    let uv = in.uv - center;

    let dist = length(uv) * 2.0;
    let angle = atan2(uv.y, uv.x) + PI; // 0 to TWO_PI

    // Ring parameters
    let radius = 0.8;
    let thickness = 0.15;
    let inner_radius = radius - thickness;

    // Outside ring = transparent
    if dist < inner_radius || dist > radius {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    // Simple rotating arc - 270 degrees (3/4 of circle)
    let arc_length = PI * 1.5;
    let rotation = uniforms.time * 5.0;
    let arc_start = rotation - floor(rotation / TWO_PI) * TWO_PI;

    // Check if angle is in the arc
    var diff = angle - arc_start;
    if diff < 0.0 {
        diff = diff + TWO_PI;
    }
    let in_arc = diff < arc_length;

    // Anti-aliasing
    let edge = 0.02;
    let aa = (1.0 - smoothstep(radius - edge, radius, dist)) * smoothstep(inner_radius, inner_radius + edge, dist);

    if in_arc {
        return vec4<f32>(uniforms.bar_color.rgb, uniforms.bar_color.a * aa);
    } else {
        return vec4<f32>(uniforms.track_color.rgb, uniforms.track_color.a * aa);
    }
}
