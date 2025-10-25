struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) world_position: vec3f,
    @location(1) normal: vec3f,
    @location(2) tex_coord: vec2f,
    @location(3) @interpolate(flat) layer_idx: u32,
    @location(4) @interpolate(flat) bone_idx: u32,
};

struct PushConstants {
    camera_proj_view: mat4x4f,
}
var<push_constant> push_constants: PushConstants;

// @group(0) @binding(0)
// var<uniform> camera_view: mat4x4f;
// @group(0) @binding(1)
// var<uniform> camera_proj: mat4x4f;
// @group(0) @binding(2)
// var<uniform> camera_pos: vec3f;
@group(0) @binding(0)
var<uniform> entity_mvp: array<mat4x4f, 64>; // make sure to match the max bone count

// vertex shader
@vertex
fn vs_main(
    @location(0) world_position: vec3f,
    @location(1) normal: vec3f,
    @location(2) tex_coord: vec2f,
    @location(3) @interpolate(flat) layer_idx: u32,
    @location(4) @interpolate(flat) bone_idx: u32,
) -> VertexOut {
    var output: VertexOut;

    let model_view = entity_mvp[bone_idx];

    output.position = push_constants.camera_proj_view * model_view * vec4(world_position, 1.0);

    output.world_position = world_position;
    output.normal = normal;
    output.tex_coord = tex_coord;
    output.layer_idx = layer_idx;
    output.bone_idx = bone_idx;

    return output;
}

// fragment shader
@group(1) @binding(0) var mipmap: texture_2d_array<f32>;
@group(1) @binding(1) var palette: texture_2d<f32>;
@group(1) @binding(2) var nearest_sampler: sampler;

fn calculate_base_color(
    position: vec4f,
    normal: vec3f,
    tex_coord: vec2f,
    layer_idx: u32,
    bone_idx: u32,
) -> vec4f {
    var albedo: vec4f;
    let palette_index = textureSample(mipmap, nearest_sampler, tex_coord, layer_idx).r;
    let palette_uv = vec2<f32>(palette_index, f32(layer_idx));

    albedo = textureSample(palette, nearest_sampler, palette_uv);
    
    // albedo = bicubic_filtering(tex_coord, layer_idx);
    // albedo = nearest_aa_filtering(tex_coord, layer_idx);
    // albedo = pixel_art_filter2(tex_coord, layer_idx);

    // this is mdl vertex
    let alpha = albedo.a;

    // pre multiply
    var final_color = albedo.rgb * alpha;

    // light is always pointing down
    let normal_z = (normal.z + 1.0) / 2.0;

    // TODO: texture flags
    // let texture_flags = data_b[0];

    // // if not flatshade, don't do shading
    // if (texture_flags & 1u) == 0 {
    //     final_color = final_color * normal_z;
    // }

    // // masked
    // if (texture_flags & (1u << 6)) != 0 {
    //     final_color = alpha_test(tex_coord, layer_idx, final_color, alpha);
    // }

    // // additive
    // if (texture_flags & (1u << 5)) != 0 {

    // }

    // need to repeat it because we also want to filter othre stuffs we don't want to draw like nodraw :()
    // if full_bright {
    //     return albedo;
    // }

    return vec4(final_color, alpha);

    return albedo;
}

@fragment
fn fs_main(
    @builtin(position) position: vec4f,
    @location(0) world_position: vec3f,
    @location(1) normal: vec3f,
    @location(2) tex_coord: vec2f,
    @location(3) @interpolate(flat) layer_idx: u32,
    @location(4) @interpolate(flat) bone_idx: u32,
) -> @location(0) vec4f {
    let color = calculate_base_color(position, normal, tex_coord, layer_idx, bone_idx);

    return color;
}