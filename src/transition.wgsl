@group(0) @binding(0) var input_texture : texture_2d<f32>;
@group(0) @binding(1) var output_texture : texture_storage_2d<rg32float, write>;
var<workgroup> neighbors: array<array<vec4<f32>, 16>, 16>;

@compute
@workgroup_size(16, 16)
fn main(
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    // Compute global texture coords
    var coords: vec2<u32> = workgroup_id.xy * vec2<u32>(14, 14) + local_id.xy - vec2<u32>(1, 1);
    coords.x = coords.x % textureDimensions(input_texture).x;
    coords.y = coords.y % textureDimensions(input_texture).y;

    // Read the neighbors into shared memory
    neighbors[local_id.x][local_id.y] = textureLoad(input_texture, coords, 0);
    workgroupBarrier(); // wait for all threads in the workgroup to finish

    // Discard the threads around the edges
    if local_id.x == 0 | local_id.x == 15 | local_id.y == 0 | local_id.y == 15 {
        return;
    }

    // Sum neighbors
    var color: vec4<f32> = neighbors[local_id.x][local_id.y];
    var count: u32 = 0;
    count += count_neighbor(local_id.xy, vec2<i32>( 1, 1));
    count += count_neighbor(local_id.xy, vec2<i32>( 0, 1));
    count += count_neighbor(local_id.xy, vec2<i32>(-1, 1));
    count += count_neighbor(local_id.xy, vec2<i32>( 1, 0));
    count += count_neighbor(local_id.xy, vec2<i32>(-1, 0));
    count += count_neighbor(local_id.xy, vec2<i32>( 1,-1));
    count += count_neighbor(local_id.xy, vec2<i32>( 0,-1));
    count += count_neighbor(local_id.xy, vec2<i32>(-1,-1));

    // Apply Game of Life rules
    if (count == 3) {
        color.r = 1.0;
    } else if (count != 2) {
        color.r = -1.0;
    }

    // Set green channel for display
    color.g = max(color.r, color.g * 0.99);

    textureStore(output_texture, coords, color);
}

fn count_neighbor(coords: vec2<u32>, offset: vec2<i32>) -> u32 {
    return u32(neighbors[i32(coords.x) + offset.x][i32(coords.y) + offset.y].r > 0.0);
}