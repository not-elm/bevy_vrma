use crate::vrm::mtoon::{MToonMaterial, MToonMaterialKey};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey, MeshPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    CompareFunction, Face, RenderPipelineDescriptor, SpecializedMeshPipeline,
    SpecializedMeshPipelineError,
};

#[derive(Resource)]
pub(super) struct MToonOutlinePipeline {
    pub base: MaterialPipeline,
}

impl FromWorld for MToonOutlinePipeline {
    fn from_world(world: &mut World) -> Self {
        Self {
            base: world.resource::<MaterialPipeline>().clone(),
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
        // First specialize the base mesh pipeline
        let mut descriptor = self.base.mesh_pipeline.specialize(key.mesh_key, layout)?;

        // Then apply the material-specific specialization
        let material_key = MaterialPipelineKey {
            mesh_key: key.mesh_key,
            bind_group_data: key.bind_group_data,
        };
        MToonMaterial::specialize(&self.base, &mut descriptor, layout, material_key)?;

        // Finally, apply outline-specific modifications
        descriptor.label.replace("mtoon_outline_pipeline".into());
        descriptor.vertex.shader_defs.push(PASS_NAME.into());
        if let Some(stencil) = descriptor.depth_stencil.as_mut() {
            // Avoid drawing backfaces that sit at the same depth as the front faces.
            // This reduces full-surface outline fills on thin meshes.
            stencil.depth_compare = CompareFunction::Greater;
        }
        descriptor.primitive.cull_mode.replace(Face::Front);
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.push(PASS_NAME.into());
        }
        Ok(descriptor)
    }
}
