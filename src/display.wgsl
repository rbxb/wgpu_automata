@group(0) @binding(0) var texture : texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vec2<f32>(model.position.xy);
    out.clip_position = vec4<f32>(model.position.xy * 2 - 1, 0.0, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(texture, texture_sampler, in.uv);
    return max(mix(
        vec4<f32>(0.0, -0.4, 0.0, 1.0),
        vec4<f32>(0.0, 1.2, 0.5, 1.0),
        color.g
    ), vec4<f32>(color.r, color.r, color.r, 1.0));
}
