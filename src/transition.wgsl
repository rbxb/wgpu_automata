@group(0) @binding(0) var input_texture : texture_2d<u32>;
@group(1) @binding(0) var output_texture : texture_storage_2d<r32uint, write>;

@compute
@workgroup_size(16, 16)
fn main(
    @builtin(global_invocation_id) global_id : vec3<u32>,
) {
    let dimensions = textureDimensions(input_texture);
    let coords = vec2<u32>(global_id.xy);

    if(coords.x >= dimensions.x || coords.y >= dimensions.y) {
        return;
    }

    let color = textureLoad(input_texture, coords.xy, 0);
    textureStore(output_texture, coords.xy, color);
}