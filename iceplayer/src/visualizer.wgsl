// PlasmaGlobe Audio Visualizer
//
// Based on work by:
// - nimitz (https://www.shadertoy.com/view/XsjXRm) - Original PlasmaGlobe
// - ArthurTent (https://github.com/ArthurTent/ShaderAmp) - Audio-reactive modifications
// - Dave_Hoskins (https://www.shadertoy.com/view/4djSRW) - Hash functions (MIT License)
// - BigWings - Background effects
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

const PI: f32 = 3.14159265359;
const NUM_RAYS: f32 = 25.0;
const VOLUMETRIC_STEPS: i32 = 19;
const MAX_ITER: i32 = 35;
const FAR: f32 = 6.0;

// Global state
var<private> snd: f32;
var<private> s_vec: vec4<f32>;

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

fn FFT(a: i32) -> f32 {
    let idx = u32(clamp(a, 0, 63));
    return pow(get_band(idx), 5.0);
}

// MIT Licensed hash From Dave_Hoskins (https://www.shadertoy.com/view/4djSRW)
fn hash33(p_in: vec3<f32>) -> vec3<f32> {
    var p = fract(p_in * vec3<f32>(443.8975, 397.2973, 491.1871));
    p += dot(p.zxy, p.yxz + 19.27);
    return fract(vec3<f32>(p.x * p.y, p.z * p.x, p.y * p.z));
}

fn hash_f(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453);
}

fn stars(p_in: vec3<f32>) -> vec3<f32> {
    var c = vec3<f32>(0.0);
    let res = uniforms.resolution.x * 0.8;
    var p = p_in;

    for (var i = 0.0; i < 4.0; i += 1.0) {
        let q = fract(p * (0.15 * res)) - 0.5;
        let id = floor(p * (0.15 * res));
        let rn = hash33(id).xy;
        var c2 = 1.0 - smoothstep(0.0, 0.6, length(q));
        c2 *= step(rn.x, 0.0005 + i * i * 0.001);
        c += c2 * (mix(vec3<f32>(1.0, 0.49, 0.1), vec3<f32>(0.75, 0.9, 1.0), rn.y) * 0.25 + 0.75);
        p *= 1.4;
    }
    return c * c * 0.65;
}

fn mm2(a: f32) -> mat2x2<f32> {
    let c = cos(a);
    let s = sin(a);
    return mat2x2<f32>(c, -s, s, c);
}

fn noise_1d(x: f32) -> f32 {
    return textureSampleLevel(noise_texture, noise_sampler, vec2<f32>(x * 0.01, 0.5), 0.0).x;
}

fn noise_3d(p: vec3<f32>) -> f32 {
    let ip = floor(p);
    var fp = fract(p);
    fp = fp * fp * (3.0 - 2.0 * fp);

    let tap = (ip.xy + vec2<f32>(37.0, 17.0) * ip.z) + fp.xy;
    let rg = textureSampleLevel(noise_texture, noise_sampler, (tap + 0.5) / 256.0, 0.0).yx;
    return mix(rg.x, rg.y, fp.z);
}

const m3: mat3x3<f32> = mat3x3<f32>(
    0.00, 0.80, 0.60,
    -0.80, 0.36, -0.48,
    -0.60, -0.48, 0.64
);

fn fsnd() -> f32 {
    s_vec = vec4<f32>(
        get_band(0u),
        get_band(16u),
        get_band(32u),
        get_band(48u)
    );

    var h = s_vec.x;
    h += s_vec.y * 0.5;
    h += s_vec.z * 0.3;
    h += s_vec.w * 0.2;
    return h;
}

fn Background(uv: vec2<f32>) -> vec3<f32> {
    let d = length(uv - vec2<f32>(0.0, 0.2));
    var col = vec3<f32>(1.0, 0.4, 0.3);
    col *= smoothstep(0.8, 0.0, d) * 1.5 * (FFT(50) * 0.5);
    return col;
}

fn flow(p_in: vec3<f32>, t: f32) -> f32 {
    let time_val = uniforms.time.x * 1.1;
    var z = 2.0;
    var rz = 0.0;
    let bp = p_in;
    var p = p_in;

    for (var i = 1.0; i < 5.0; i += 1.0) {
        p += time_val * 0.1;
        rz += (sin(noise_3d(p + t * 0.8) * 6.0) * 0.5 + 0.5) / z;
        p = mix(bp, p, 0.6);
        z *= 2.0;
        p *= 2.01;
        p = m3 * p;
    }
    return rz;
}

fn sins(x_in: f32) -> f32 {
    let time_val = uniforms.time.x * 1.1;
    var rz = 0.0;
    var z = 2.0;
    var x = x_in;

    for (var i = 0.0; i < 3.0; i += 1.0) {
        rz += abs(fract(x * 1.4) - 0.5) / z;
        x *= 1.3;
        z *= 1.15;
        x -= time_val * 0.65 * z;
    }
    return rz;
}

fn segm(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h) * 0.5;
}

fn path(i: f32, d: f32) -> vec3<f32> {
    var en = vec3<f32>(0.0, 0.0, 1.0);
    let sns2 = sins(d + i * 0.5) * 0.22;
    let sns = sins(d + i * 0.6) * 0.21;
    let rot1 = mm2((hash_f(i * 10.569) - 0.5) * 6.2 + sns2);
    let rot2 = mm2((hash_f(i * 4.732) - 0.5) * 6.2 + sns);
    en = vec3<f32>(rot1 * en.xz, en.y).xzy;
    en = vec3<f32>(rot2 * en.xy, en.z);
    return en;
}

fn map(p: vec3<f32>, i: f32) -> vec2<f32> {
    let lp = length(p);
    let bg = vec3<f32>(0.0);
    let en = path(i, lp);

    let ins = smoothstep(0.11, 0.46, lp);
    let outs = 0.15 + smoothstep(0.0, 0.15, abs(lp - 1.0));
    let p_scaled = p * ins * outs;
    let id = ins * outs;

    let rz = segm(p_scaled, bg, en) - 0.011;
    return vec2<f32>(rz, id);
}

fn march(ro: vec3<f32>, rd: vec3<f32>, startf: f32, maxd: f32, j: f32) -> f32 {
    let precis = 0.001;
    var h = 0.5;
    var d = startf;

    for (var i = 0; i < MAX_ITER; i++) {
        if abs(h) < precis || d > maxd {
            break;
        }
        d += h * 1.2;
        let idx = u32(clamp(i, 0, 63));
        let res = map(ro + rd * d, j).x * get_band(idx) * 1.5;
        h = res;
    }
    return d;
}

fn vmarch(ro: vec3<f32>, rd: vec3<f32>, j: f32, orig: vec3<f32>) -> vec3<f32> {
    var p = ro;
    var sum = vec3<f32>(0.0);
    let fsnd_val = fsnd();

    for (var i = 0; i < VOLUMETRIC_STEPS; i++) {
        let r = map(p, j);
        p += rd * 0.03;
        let lp = length(p);

        var col = sin(vec3<f32>(1.05, 2.5, 1.52) * 3.94 + r.y) * 0.85 + 0.4 * fsnd_val;
        col *= smoothstep(0.0, 0.015, -r.x);
        col *= smoothstep(0.04, 0.2, abs(lp - 1.1));
        col *= smoothstep(0.1, 0.34, lp);
        let dist_factor = log(distance(p, orig) - 2.0) + 0.75;
        let noise_factor = 1.2 - noise_3d(vec3<f32>(lp * 2.0 + j * 13.0 + uniforms.time.x * 5.5, 0.0, 0.0)) * 1.1;
        sum += abs(col) * 5.0 * noise_factor / max(dist_factor, 0.1);
    }
    return sum * fsnd_val;
}

fn iSphere2(ro: vec3<f32>, rd: vec3<f32>) -> vec2<f32> {
    let oc = ro;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - 1.0;
    let h = b * b - c;
    if h < 0.0 {
        return vec2<f32>(-1.0);
    }
    return vec2<f32>(-b - sqrt(h), -b + sqrt(h));
}

fn camera(fragCoord: vec2<f32>) -> array<vec3<f32>, 2> {
    let time_val = uniforms.time.x;
    var rd = normalize(vec3<f32>(fragCoord, 1.0));
    var ro = vec3<f32>(0.0, 0.0, -15.0);

    let ff = min(1.0, step(0.001, 0.0) + step(0.001, 0.0)) + sin(time_val / 20.0);
    var m = PI * ff + vec2<f32>(0.1 / uniforms.resolution.x, 0.1 / uniforms.resolution.y) * (PI * 2.0);
    m.y = sin(m.y * 0.5) * 0.3 + 0.5;

    let sm = sin(m) * (1.0 + sin(time_val / 10.0) / 2.0);
    let cm = cos(m);
    let rotX = mat3x3<f32>(
        1.0, 0.0, 0.0,
        0.0, cm.y, sm.y,
        0.0, -sm.y, cm.y
    );
    let rotY = mat3x3<f32>(
        cm.x, 0.0, -sm.x,
        0.0, 1.0, 0.0,
        sm.x, 0.0, cm.x
    );

    let t = rotY * rotX;
    ro = t * ro;
    rd = t * rd;
    rd = normalize(rd);

    return array<vec3<f32>, 2>(ro, rd);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let time_val = uniforms.time.x * 1.1;

    // Calculate sound
    var snd_local = 0.0;
    let max_freq = 100;
    for (var i = 1; i < max_freq; i++) {
        snd_local += FFT(i) * f32(i);
    }
    snd_local /= f32(max_freq * 20);
    snd = snd_local;

    let uv = in.uv * 2.0 - 1.0;

    // Camera + rd for stars
    var rd = normalize(vec3<f32>(uv, -1.5));
    let cam = camera(uv);
    rd.x += sin(time_val / 1000.0) * 2.0;
    var bg = stars(rd) * (1.0 + 30.0 * snd_local);

    let fragCoord = in.uv * uniforms.resolution.xy;
    var p = fragCoord / uniforms.resolution.xy - 0.5;
    p.x *= uniforms.resolution.x / uniforms.resolution.y;

    // Camera
    var ro = vec3<f32>(0.0, 0.0, 5.0);
    rd = normalize(vec3<f32>(p * 0.7, -1.5));
    let mx = mm2(time_val * 0.4);
    let my = mm2(time_val * 0.3);
    ro = vec3<f32>(mx * ro.xz, ro.y).xzy;
    rd = vec3<f32>(mx * rd.xz, rd.y).xzy;
    ro = vec3<f32>(my * ro.xy, ro.z);
    rd = vec3<f32>(my * rd.xy, rd.z);

    let bro = ro;
    let brd = rd;

    var col = vec3<f32>(0.0125, 0.0, 0.025);

    // Ray marching loop
    for (var j = 1.0; j < NUM_RAYS; j += 1.0) {
        let audio_mod = get_band(u32(j) % 64u);
        if j >= NUM_RAYS * audio_mod {
            continue;
        }

        ro = bro;
        rd = brd;
        let mm = mm2((time_val * 0.1 + ((j + 1.0) * 5.1)) * j * 0.25);
        ro = vec3<f32>(mm * ro.xy, ro.z);
        rd = vec3<f32>(mm * rd.xy, rd.z);
        ro = vec3<f32>(mm * ro.xz, ro.y).xzy;
        rd = vec3<f32>(mm * rd.xz, rd.y).xzy;

        let rz = march(ro, rd, 2.5, FAR, j);
        if rz >= FAR {
            continue;
        }
        let pos = ro + rz * rd;
        col = max(col, vmarch(pos, rd, j, bro));
    }

    ro = bro;
    rd = brd;
    let sph = iSphere2(ro, rd);

    if sph.x > 0.0 {
        let pos = ro + rd * sph.x;
        let pos2 = ro + rd * sph.y;
        let rf = reflect(rd, pos);
        let rf2 = reflect(rd, pos2);
        let nz = -log(abs(flow(rf * 1.2, time_val) - 0.01));
        let nz2 = -log(abs(flow(rf2 * 1.2, -time_val) - 0.01));
        col += (0.1 * nz * nz * vec3<f32>(0.12, 0.12, 0.5) + 0.05 * nz2 * nz2 * vec3<f32>(0.55, 0.2, 0.55)) * 0.8;
    }

    let p_bg = vec2<f32>(p.x, -p.y);
    let bg_col = Background(p_bg);
    col *= (0.3 + bg_col * 10.5);
    col += bg_col;

    var final_col = col * 1.3;
    final_col += bg;

    return vec4<f32>(final_col, 1.0);
}
