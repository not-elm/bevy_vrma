use crate::prelude::*;
use crate::vrm::humanoid_bone::HumanoidBoneRegistry;
use bevy::animation::AnimationTargetId;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Per-bone-entity component storing retarget rotation transformations keyed by `AnimationNodeIndex`.
/// Automatically cleaned up via `despawn_recursive`.
#[derive(Component, Default, Clone, Debug, Deref, DerefMut)]
pub(crate) struct RetargetRotationTable(pub HashMap<AnimationNodeIndex, Transformation>);

pub(crate) fn compute_rotation_transformations(
    vrma: Entity,
    node_index: AnimationNodeIndex,
    root_bone: Entity,
    registry: &HumanoidBoneRegistry,
    searcher: &ChildSearcher,
    bones: &Query<(&RestTransform, &RestGlobalTransform, &AnimationTargetId)>,
) -> Vec<(Entity, AnimationNodeIndex, Transformation)> {
    let mut result = Vec::new();
    for (bone, name) in registry.iter() {
        let Some(vrma_bone_entity) = searcher.find_from_name(vrma, name) else {
            continue;
        };
        let Some(rig_bone_entity) = searcher.find_by_bone_name(root_bone, bone) else {
            continue;
        };
        let Some((rest, rest_g, _)) = bones.get(rig_bone_entity).ok() else {
            continue;
        };
        let Some((vrma_rest, vrma_rest_g, _)) = bones.get(vrma_bone_entity).ok() else {
            continue;
        };
        let transformation = Transformation {
            src_rest: vrma_rest.0.rotation,
            src_rest_g: vrma_rest_g.0.rotation(),
            dist_rest: rest.0.rotation,
            dist_rest_g: rest_g.0.rotation(),
        };
        result.push((rig_bone_entity, node_index, transformation));
    }
    result
}

#[derive(Debug, Copy, Clone, Reflect)]
pub(crate) struct Transformation {
    src_rest: Quat,
    src_rest_g: Quat,
    dist_rest: Quat,
    dist_rest_g: Quat,
}

impl Transformation {
    pub fn transform(
        &self,
        src_pose: Quat,
    ) -> Quat {
        // https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm_animation-1.0/how_to_transform_human_pose.md
        let normalized_local_rotation =
            self.src_rest_g * self.src_rest.inverse() * src_pose * self.src_rest_g.inverse();
        self.dist_rest * self.dist_rest_g.inverse() * normalized_local_rotation * self.dist_rest_g
    }
}
