@group(0) @binding(0) var input_texture : texture_2d<f32>;
@group(0) @binding(1) var output_texture : texture_storage_2d<rg32float, write>;

@compute
@workgroup_size(16, 16)
fn main(
    @builtin(global_invocation_id) global_id : vec3<u32>,
) {
    let dimensions = textureDimensions(input_texture);
    let coords = vec2<u32>(global_id.xy);

    if (coords.x >= dimensions.x || coords.y >= dimensions.y) {
        return;
    }

    var color: vec4<f32> = textureLoad(input_texture, coords, 0);
    let previous_state = color.r;
    
    var count: u32 = 0;
    count += count_neighbor_wrap(coords, -1, -1);
    count += count_neighbor_wrap(coords, 0, -1);
    count += count_neighbor_wrap(coords, 1, -1);
    count += count_neighbor_wrap(coords, -1, 0);
    count += count_neighbor_wrap(coords, 1, 0);
    count += count_neighbor_wrap(coords, -1, 1);
    count += count_neighbor_wrap(coords, 0, 1);
    count += count_neighbor_wrap(coords, 1, 1);
    
    if (count == 3) {
        color.r = 1.0;
    } else if (count != 2) {
        color.r = -1.0;
    } else {
        color.r = previous_state;
    }

    if (color.r > 0) {
        color.g = 1.0;
    } else {
        color.g *= 0.9;
    }

    textureStore(output_texture, coords, color);
}

fn count_neighbor_wrap(coords: vec2<u32>, offset_x: i32, offset_y: i32) -> u32 {
    let dimensions = textureDimensions(input_texture);
    let neighbor = vec2<i32>(
        (i32(dimensions.x + coords.x) + offset_x) % i32(dimensions.x),
        (i32(dimensions.y + coords.y) + offset_y) % i32(dimensions.y)
    );
    return u32(textureLoad(input_texture, neighbor, 0).r > 0.0);
} 