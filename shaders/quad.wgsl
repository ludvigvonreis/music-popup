struct GlobalUniforms {
    viewport: vec2<f32>, // width, height
};

@group(0) @binding(0)
var<uniform> u: GlobalUniforms;

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
    @location(2) instance_size: vec2<f32>,
    //@location(3) viewport: vec2<f32>,
};

@vertex
fn vs_main(vertex: Vertex, instance: Instance) -> VertexOutput {
    var out: VertexOutput;

	let scale: vec2<f32> = (instance.instance_size / u.viewport);

	// Apply size to the vertex shape
	let scaled_vertex = vertex.position * scale;

	// Now add instance offset
	let final_pos = scaled_vertex + instance.instance_pos;

	out.clip_position = vec4<f32>(final_pos, 0.0, 1.0);

	out.uv = vertex.position * (instance.instance_size);

    out.color = instance.instance_color;
	out.instance_size = instance.instance_size;

    return out;
}



fn sdRoundedBox2D(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = 10.0;
    let halfSize = in.instance_size / 2;

    let p = in.uv * 0.5;
    let d = sdRoundedBox2D(p, halfSize - radius, radius);

    // simple anti-aliased color
    var color = in.color.xyz;
    let alpha = smoothstep(0.0, 1.0, -d); // negative inside
    
	return vec4<f32>(color, alpha);
}