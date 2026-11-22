struct GlobalUniforms {
    viewport: vec2<f32>,
};

struct ObjectUniform {
    position: vec2<f32>,
    size: vec2<f32>,
    color: vec4<f32>,
    border_color: vec4<f32>,
    shadow_color: vec4<f32>,
    shadow_offset: vec2<f32>,
    radius: f32,
    border_width: f32,
    shadow_blur: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> u: GlobalUniforms;
@group(1) @binding(0) var<storage, read> objects: array<ObjectUniform>;

struct Vertex {
    @location(0) position: vec2<f32>,
    @builtin(instance_index) instanceIndex: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) border_color: vec4<f32>,
    @location(2) shadow_color: vec4<f32>,
    @location(3) local: vec2<f32>,         // pixels from rect center
    @location(4) size: vec2<f32>,
    @location(5) shadow_offset: vec2<f32>,
    @location(6) params: vec3<f32>,        // radius, border_width, shadow_blur
};

@vertex
fn vs_main(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let inst = objects[vertex.instanceIndex];

    // Grow the quad so the shadow has room to draw outside the rect.
    var margin = vec2<f32>(1.0);
    if inst.shadow_color.a > 0.0 {
        margin += vec2<f32>(inst.shadow_blur) + abs(inst.shadow_offset);
    }
    let quad_size = inst.size + margin * 2.0;

    let center = vec2<f32>(
        (inst.position.x + inst.size.x * 0.5) / (u.viewport.x * 0.5) - 1.0,
        1.0 - (inst.position.y + inst.size.y * 0.5) / (u.viewport.y * 0.5)
    );
    out.clip_position = vec4<f32>(vertex.position * (quad_size / u.viewport) + center, 0.0, 1.0);

    out.local = vertex.position * quad_size * 0.5;
    out.size = inst.size;
    out.color = inst.color;
    out.border_color = inst.border_color;
    out.shadow_color = inst.shadow_color;
    out.shadow_offset = inst.shadow_offset;
    out.params = vec3<f32>(inst.radius, inst.border_width, inst.shadow_blur);
    return out;
}

fn sdRoundedBox2D(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        return c / 12.92;
    } else {
        return pow((c + 0.055) / 1.055, 2.4);
    }
}

fn srgb_vec_to_linear(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(srgb_to_linear(c.x), srgb_to_linear(c.y), srgb_to_linear(c.z));
}

// sRGB straight-alpha color -> linear premultiplied.
fn premul(c: vec4<f32>, coverage: f32) -> vec4<f32> {
    let a = c.a * coverage;
    return vec4<f32>(srgb_vec_to_linear(c.rgb) * a, a);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let half_size = in.size * 0.5;
    let r = min(in.params.x, min(half_size.x, half_size.y));
    let bw = min(in.params.y, min(half_size.x, half_size.y));
    let blur = in.params.z;

    // Outer edge.
    let b = half_size - vec2<f32>(r);
    let d = sdRoundedBox2D(in.local, b, r);
    let outer = clamp(0.5 - d, 0.0, 1.0);

    // Inner edge: the same shape inset by the border width.
    let r_in = max(r - bw, 0.0);
    let d_in = sdRoundedBox2D(in.local, half_size - vec2<f32>(bw + r_in), r_in);
    let inner = clamp(0.5 - d_in, 0.0, 1.0);

    // Background fills the whole outer shape; the border is drawn over it.
    let fill = premul(in.color, outer);
    let border = premul(in.border_color, outer - inner);
    var result = border + fill * (1.0 - border.a);

    // Shadow: the same rounded box, offset and blurred, under everything.
    if in.shadow_color.a > 0.0 {
        let ds = sdRoundedBox2D(in.local - in.shadow_offset, b, r);
        var cov = clamp(0.5 - ds, 0.0, 1.0);
        if blur > 0.0 {
            cov = 1.0 - smoothstep(-blur * 0.5, blur * 0.5, ds);
        }
        result = result + premul(in.shadow_color, cov) * (1.0 - result.a);
    }

    return result;
}
