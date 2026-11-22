struct GlobalUniforms {
    viewport: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: GlobalUniforms;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VsIn {
    @location(0) corner: vec2<f32>,     // unit quad, -1..1
    @location(1) pos: vec2<f32>,        // top-left, pixels
    @location(2) size: vec2<f32>,
    @location(3) uv_rect: vec4<f32>,    // x, y, w, h in 0..1
    @location(4) tint: vec4<f32>,       // sRGB, like RectInstance.color
    @location(5) radius: f32,
};

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) local: vec2<f32>,      // pixels from rect center
    @location(2) size: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) radius: f32,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;

    // Same placement math as quad.wgsl.
    let scale = in.size / u.viewport;
    let center = vec2<f32>(
        (in.pos.x + in.size.x * 0.5) / (u.viewport.x * 0.5) - 1.0,
        1.0 - (in.pos.y + in.size.y * 0.5) / (u.viewport.y * 0.5)
    );
    out.clip_position = vec4<f32>(in.corner * scale + center, 0.0, 1.0);

    let t = (in.corner + vec2<f32>(1.0)) * 0.5;    // 0..1, (0,0) = top-left corner of quad
    // corner.y = +1 lands at the top of the screen, so flip v for the texture.
    out.uv = in.uv_rect.xy + vec2<f32>(t.x, 1.0 - t.y) * in.uv_rect.zw;

    out.local = in.corner * in.size * 0.5;
    out.size = in.size;
    out.tint = in.tint;
    out.radius = in.radius;
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
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let half_size = in.size * 0.5;
    let d = sdRoundedBox2D(in.local, half_size - in.radius, in.radius);
    let coverage = smoothstep(0.0, 1.0, -d);

    // The texture is Rgba8UnormSrgb, so sampling already returns linear.
    // The tint is sRGB (same convention as rect colors), so convert it.
    let texel = textureSample(tex, samp, in.uv);
    let tint_lin = srgb_vec_to_linear(in.tint.rgb);

    let rgb = texel.rgb * tint_lin;
    let alpha = texel.a * in.tint.a * coverage;
    return vec4<f32>(rgb * alpha, alpha);   // premultiplied, matches quad.wgsl
}
