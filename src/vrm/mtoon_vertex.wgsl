#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    mesh_functions::{
        mesh_normal_local_to_world,
    },
    skinning,
    morph::morph,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::globals,
}
#import mtoon::types::{
    MToonMaterialUniform,
    material,
    outline_width_multiply_texture,
    outline_width_multiply_sampler,
    OUTLINE_WIDTH_MULTIPLY_TEXTURE,
    uv_animation_mask_texture,
    uv_animation_mask_sampler,
    UV_ANIMATION_MASK_TEXTURE,
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph);
#else
    var vertex = vertex_no_morph;
#endif

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

#ifdef SKINNED
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_no_morph.instance_index
    );
#else
    var world_from_local = mesh_world_from_local;
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index
    );
#endif
#endif

#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
#ifdef OUTLINE_PASS
    let animated_uv = calc_animated_uv((material.uv_transform * vec3(vertex.uv, 1.0)).xy);
    let outline_width = outline_width(animated_uv);
    let outline_normal = normalize(out.world_normal.xyz);
    out.world_position = vec4(out.world_position.xyz + outline_normal * outline_width, 1.0);
#endif
    out.position = position_world_to_clip(out.world_position.xyz);
#endif

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index
    );
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index, mesh_world_from_local[3]);
#endif

    return out;
}

fn outline_width(uv: vec2<f32>) -> f32{
    let w = material.outline_width_factor;
    if ((material.flags & OUTLINE_WIDTH_MULTIPLY_TEXTURE) != 0) {
        let texel = textureSampleLevel(outline_width_multiply_texture, outline_width_multiply_sampler, uv, 0.0);
        return w * texel.g;
    } else {
        return w;
    }
}

fn calc_animated_uv(uv: vec2<f32>) -> vec2<f32>{
    let time = calc_uv_time(uv);
    let translate = time * vec2(material.uv_animation_scroll_speed_x, material.uv_animation_rotation_speed_y);
    let rotate_rad = fract(time * material.uv_animation_rotation_speed);
    let cos_rotate = cos(rotate_rad);
    let sin_rotate = sin(rotate_rad);
    let pivot = vec2<f32>(0.5, 0.5);
    return mat2x2(cos_rotate, -sin_rotate, sin_rotate, cos_rotate) * (uv - pivot) + pivot + translate;
}

fn calc_uv_time(uv: vec2<f32>) -> f32{
    if((material.flags & UV_ANIMATION_MASK_TEXTURE) != 0u) {
        let mask = textureSampleLevel(uv_animation_mask_texture, uv_animation_mask_sampler, uv, 0.0).b;
        return mask * globals.time;
    }else{
        return globals.time;
    }
}

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex) -> Vertex {
    var vertex = vertex_in;
    let first_vertex = mesh[vertex.instance_index].first_vertex_index;
    let vertex_index = vertex.index - first_vertex;

    let weight_count = bevy_pbr::morph::layer_count();
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = bevy_pbr::morph::weight_at(i);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph(vertex_index, bevy_pbr::morph::position_offset, i);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph(vertex_index, bevy_pbr::morph::normal_offset, i);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * morph(vertex_index, bevy_pbr::morph::tangent_offset, i), 0.0);
#endif
    }
    return vertex;
}
#endif
