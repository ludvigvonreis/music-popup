struct Vertex {
    @location(0) position: vec2<f32>, // local quad vertex [-0.5,0.5]
};

struct Instance {
    @location(1) instance_pos: vec2<f32>,
    @location(2) instance_size: vec2<f32>,
    @location(3) instance_color: vec4<f32>,
    @location(4) texture_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
	@location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(vertex: Vertex, instance: Instance) -> VertexOutput {
    var out: VertexOutput;

    // Apply size and position
    let scaled_pos = vertex.position * instance.instance_size + instance.instance_pos;

    out.clip_position = vec4<f32>(scaled_pos, 0.0, 1.0);
    out.color = instance.instance_color;

	out.uv = (vertex.position.xy + vec2f(1.0, 1.0)) * 0.5;

    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    let uv = input.uv;           // 0..1 inside quad
    let radius: f32 = 0.1;       // corner radius
    let center = vec2f(0.5, 0.5);

    // Shift uv to quad space centered at center
    let p = uv - center;

    // Half-size of quad (since uv goes 0..1, half-size = 0.5)
    let half_size = vec2f(0.5, 0.5);

    // Distance from rectangle edges minus radius
    let d = abs(p) - (half_size - vec2f(radius, radius));
    
    // Take max distance outside rectangle
    let dist = max(d.x, d.y);

    // Smooth alpha at corners, fully opaque inside
    let alpha = smoothstep(0.0, 0.01, -dist) * 0.5; // 0.5 base opacity

    return vec4f(input.color.rgb, alpha);
}