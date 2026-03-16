struct GlobalUniforms {
    viewport: vec2<f32>, // width, height
};

struct ObjectUniform {
    position: vec2<f32>,
    size: vec2<f32>,
    color: vec4<f32>,
    //texture_index: u32,
    radius: f32,
    _pad: u32,
};

@group(0) @binding(0)
var<uniform> u: GlobalUniforms;
@group(1) @binding(0)
var<storage, read> objects: array<ObjectUniform>;

struct Vertex {
    @location(0) position: vec2<f32>, // local quad vertex [-0.5,0.5]
    @builtin(instance_index) instanceIndex: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) radius: f32,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let instance = objects[vertex.instanceIndex];

    let scale: vec2<f32> = (instance.size / u.viewport);

    // Apply size to the vertex shape
    let scaled_vertex = vertex.position * scale;

    // Now add instance offset
    let clip_pos = vec2<f32>(
        (instance.position.x + instance.size.x * 0.5) / (u.viewport.x * 0.5) - 1.0,
        1.0 - (instance.position.y + instance.size.y * 0.5) / (u.viewport.y * 0.5)
    );
    let final_pos = scaled_vertex + clip_pos;
    out.clip_position = vec4<f32>(final_pos, 0.0, 1.0);

    out.uv = vertex.position * instance.size;

    out.color = instance.color;
    out.size = instance.size;
    out.radius = instance.radius;

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
    return vec3(srgb_to_linear(c.x), srgb_to_linear(c.y), srgb_to_linear(c.z));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = in.radius;
    //let radius = 10.0;
    let halfSize = in.size / 2;

    let p = in.uv * 0.5;
    let d = sdRoundedBox2D(p, halfSize - radius, radius);

    // simple anti-aliased color
    var color = in.color.xyz;
    let alpha = smoothstep(0.0, 1.0, -d) * in.color.a;
    let linear = srgb_vec_to_linear(color);
    return vec4<f32>(linear * alpha, alpha);
}
