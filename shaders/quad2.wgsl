// ── Uniforms ─────────────────────────────────────────────────────────────────

struct GlobalUniforms {
    viewport: vec2<f32>,
};

// Per-object data written by the CPU / mesh builder.
// All color channels are pre-divided by 255 (0-1 range).
// encodedOutline:
//   >= 0  →  plain outline, value = outlineSize
//   <  0  →  frame mode,   abs(value) - 1 = placement*4096 + outlineSize
//             placement: 0=inside, 1=center, 2=outside
struct ObjectUniform {
    position: vec2<f32>,
    size: vec2<f32>,
    color: vec4<f32>,

    // per-corner radii: top-right, top-left, bottom-left, bottom-right
    roundness: vec4<f32>,

    // outline / frame
    encodedOutline: f32,   // see comment above
    frameWidth: f32,   // used in frame mode

    // shadow
    shadowSize: f32,
    shadowBlur: f32,
    shadowPow: f32,
    shadowColor: vec4<f32>,

    // outline color
    outlineColor: vec4<f32>,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<uniform> u: GlobalUniforms;
@group(1) @binding(0) var<storage, read> objects: array<ObjectUniform>;

// ── Vertex stage ─────────────────────────────────────────────────────────────

struct Vertex {
    @location(0) position: vec2<f32>,   // local quad [-0.5, 0.5]
    @builtin(instance_index) instanceIndex: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) sdfPos: vec2<f32>,   // pixel-space position relative to centre
    @location(2) size: vec2<f32>,
    @location(3) roundness: vec4<f32>,
    @location(4) encodedOutline: f32,
    @location(5) frameWidth: f32,
    @location(6) shadowParams: vec3<f32>,   // x=shadowSize y=shadowBlur z=shadowPow
    @location(7) shadowColor: vec4<f32>,
    @location(8) outlineColor: vec4<f32>,
};

// Decode outline extent for vertex padding (mirrors Unity vert shader).
fn outlineExtent(encodedOutline: f32, frameWidth: f32) -> f32 {
    if encodedOutline < -0.5 {
        let absVal = abs(encodedOutline) - 1.0;
        let placement = floor(absVal / 4096.0 + 0.001);
        let olSize = absVal - placement * 4096.0;
        let outerEdge = select(
            select(frameWidth, frameWidth * 0.5, placement < 1.5),
            0.0,
            placement < 0.5
        );
        return olSize + outerEdge;
    }
    return encodedOutline;
}

@vertex
fn vs_main(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let obj = objects[vertex.instanceIndex];

    // Expand quad by the same padding as Unity so the shadow/outline aren't clipped.
    let extent = outlineExtent(obj.encodedOutline, obj.frameWidth);
    let padding = extent + obj.shadowSize + obj.shadowBlur + 1.0;
    let padded = obj.size + padding * 2.0;

    let scale = padded / u.viewport;
    let scaledPos = vertex.position * scale;
    let finalPos = scaledPos + (obj.position / (u.viewport * 0.5));

    out.clip_position = vec4<f32>(finalPos, 0.0, 1.0);
    out.color = obj.color;
    out.sdfPos = vertex.position * padded;   // pixel coords from centre
    out.size = obj.size;
    out.roundness = obj.roundness;
    out.encodedOutline = obj.encodedOutline;
    out.frameWidth = obj.frameWidth;
    out.shadowParams = vec3<f32>(obj.shadowSize, obj.shadowBlur, obj.shadowPow);
    out.shadowColor = obj.shadowColor;
    out.outlineColor = obj.outlineColor;

    return out;
}

// ── SDF helpers ──────────────────────────────────────────────────────────────

// Per-corner rounded box SDF  (matches Unity sdRoundedBox).
// r  = vec4(top-right, top-left, bottom-left, bottom-right) radii
fn sdRoundedBox(p: vec2<f32>, b: vec2<f32>, r_in: vec4<f32>) -> f32 {
    var r = r_in;
    r = select(r.zwzw, r.xyxy, vec4<bool>(p.x > 0.0));  // pick x-side pair
    r.x = select(r.y, r.x, p.y > 0.0);                  // pick y-side radius
    let q = abs(p) - b + r.x;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r.x;
}

// ── Fragment stage ────────────────────────────────────────────────────────────

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let halfSize = in.size * 0.5;
    let shadowSize = in.shadowParams.x;
    let shadowBlur = in.shadowParams.y;
    let shadowPow = in.shadowParams.z;
    let rawOutline = in.encodedOutline;

    // ── Decode frame / outline mode ───────────────────────────────────────────
    var frameMode = rawOutline < -0.5;
    var outlineSize = 0.0;
    var outerEdge = 0.0;
    var innerEdge = 0.0;

    if frameMode {
        let absVal = abs(rawOutline) - 1.0;
        let placement = floor(absVal / 4096.0 + 0.001);
        outlineSize = absVal - placement * 4096.0;
        let fw = in.frameWidth;

        if placement < 0.5 {
            outerEdge = 0.0;      innerEdge = -fw;
        } else if placement < 1.5 {
            outerEdge = fw * 0.5; innerEdge = -fw * 0.5;
        } else {
            outerEdge = fw;       innerEdge = 0.0;
        }
    } else {
        outlineSize = rawOutline;
    }

    // ── Base shape SDF ────────────────────────────────────────────────────────
    let dist = sdRoundedBox(in.sdfPos, halfSize, in.roundness);

    // ── Optionally convert to ring SDF (frame mode) ───────────────────────────
    var ringCenter = 0.0;
    var ringHalf = 0.0;
    var sdf = dist;

    if frameMode {
        ringCenter = (outerEdge + innerEdge) * 0.5;
        ringHalf = (outerEdge - innerEdge) * 0.5;
        sdf = abs(dist - ringCenter) - ringHalf;
    }

    let delta = fwidth(sdf);   // screen-space AA width

    // ── Fill ──────────────────────────────────────────────────────────────────
    let fill = 1.0 - smoothstep(0.0, delta, sdf);

    // ── Outline ───────────────────────────────────────────────────────────────
    let outline = select(
        0.0,
        1.0 - smoothstep(outlineSize, outlineSize + delta, sdf),
        outlineSize > 0.0
    );

    // ── Shadow ────────────────────────────────────────────────────────────────
    var shadow = 0.0;
    let shadowRange = shadowSize + shadowBlur;
    if shadowRange > 0.0 {
        let shadowEdge = outlineSize + shadowSize;
        shadow = 1.0 - smoothstep(shadowEdge - shadowBlur, shadowEdge + delta, sdf);
        shadow = pow(shadow, shadowPow);
    }

    // ── Emboss bevel ──────────────────────────────────────────────────────────
    var fillRgb = in.color.rgb;

    // ── Premultiplied-alpha compositing: fill over outline over shadow ─────────
    //    Matches Unity's inlined Over(fill, Over(outline, shadow))
    let masterAlpha = in.color.a;
    let gA = fill * in.color.a * masterAlpha;
    let oA = outline * in.outlineColor.a * masterAlpha;
    let sA = shadow * in.shadowColor.a * masterAlpha;

    let oneMinusGA = 1.0 - gA;
    let oneMinusOA = 1.0 - oA;

    var result: vec4<f32>;
    result.r = fillRgb.r * gA
             + in.outlineColor.r * oA * oneMinusGA
             + in.shadowColor.r * sA * oneMinusOA * oneMinusGA;
    result.g = fillRgb.g * gA
             + in.outlineColor.g * oA * oneMinusGA
             + in.shadowColor.g * sA * oneMinusOA * oneMinusGA;
    result.b = fillRgb.b * gA
             + in.outlineColor.b * oA * oneMinusGA
             + in.shadowColor.b * sA * oneMinusOA * oneMinusGA;
    result.a = gA
             + oA * oneMinusGA
             + sA * oneMinusOA * oneMinusGA;

    return result;
}
