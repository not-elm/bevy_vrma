use bevy::pbr::{
    DrawMesh, SetMaterialBindGroup, SetMeshBindGroup, SetMeshViewBindGroup,
    SetMeshViewBindingArrayBindGroup,
};
use bevy::render::render_phase::SetItemPipeline;

pub(super) type DrawOutline = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMeshBindGroup<2>,
    SetMaterialBindGroup<3>,
    DrawMesh,
);
