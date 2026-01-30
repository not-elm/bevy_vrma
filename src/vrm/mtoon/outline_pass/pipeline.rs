use crate::vrm::mtoon::{
    MTOON_FRAGMENT_SHADER_HANDLE, MTOON_VERTEX_SHADER_HANDLE, MToonMaterial, MToonMaterialKey,
};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey, MeshPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BindGroupLayoutDescriptor, CompareFunction, Face, RenderPipelineDescriptor,
    SpecializedMeshPipeline, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderDefVal;

#[derive(Resource)]
pub(super) struct MToonOutlinePipeline {
    pub base: MaterialPipeline,
    pub material_layout: BindGroupLayoutDescriptor,
}

impl FromWorld for MToonOutlinePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<bevy::render::renderer::RenderDevice>();
        let material_layout = MToonMaterial::bind_group_layout_descriptor(render_device);
        Self {
            base: world.resource::<MaterialPipeline>().clone(),
            material_layout,
        }
    }
}

/// Key for outline pipeline specialization
#[derive(Clone, Hash, PartialEq, Eq)]
pub(super) struct OutlinePipelineKey {
    pub mesh_key: MeshPipelineKey,
    pub bind_group_data: MToonMaterialKey,
}

impl SpecializedMeshPipeline for MToonOutlinePipeline {
    type Key = OutlinePipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        const PASS_NAME: &str = "OUTLINE_PASS";
        let mut descriptor = self.base.mesh_pipeline.specialize(key.mesh_key, layout)?;
        let material_key = MaterialPipelineKey {
            mesh_key: key.mesh_key,
            bind_group_data: key.bind_group_data,
        };
        MToonMaterial::specialize(&self.base, &mut descriptor, layout, material_key)?;

        descriptor.vertex.shader = MTOON_VERTEX_SHADER_HANDLE;
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader = MTOON_FRAGMENT_SHADER_HANDLE;
        }

        if descriptor.layout.len() <= 3 {
            descriptor
                .layout
                .resize(4, BindGroupLayoutDescriptor::default());
        }
        descriptor.layout[3] = self.material_layout.clone();

        descriptor.label.replace("mtoon_outline_pipeline".into());

        let material_bind_group_def = ShaderDefVal::Int("MATERIAL_BIND_GROUP".into(), 3);
        descriptor
            .vertex
            .shader_defs
            .push(material_bind_group_def.clone());
        descriptor.vertex.shader_defs.push(PASS_NAME.into());
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.depth_compare = CompareFunction::GreaterEqual;
        }
        descriptor.primitive.cull_mode.replace(Face::Front);
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.push(material_bind_group_def);
            fragment.shader_defs.push(PASS_NAME.into());
        }
        Ok(descriptor)
    }
}
