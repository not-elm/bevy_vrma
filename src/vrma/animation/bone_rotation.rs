use crate::prelude::*;
use crate::vrm::humanoid_bone::HumanoidBoneRegistry;
use bevy::animation::{
    AnimationEntityMut, AnimationEvaluationError, AnimationTargetId, animated_field,
};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::any::TypeId;
use std::fmt::{Debug, Formatter};

/// Per-bone-entity component storing retarget rotation transformations keyed by AnimationNodeIndex.
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

pub struct BoneRotationAnimationCurve {
    pub base: Box<dyn AnimationCurve>,
}

impl Debug for BoneRotationAnimationCurve {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RetargetBoneAnimationCurve").finish()
    }
}

impl AnimationCurve for BoneRotationAnimationCurve {
    fn clone_value(&self) -> Box<dyn AnimationCurve> {
        Box::new(Self {
            base: self.base.clone_value(),
        })
    }

    fn domain(&self) -> Interval {
        self.base.domain()
    }

    fn evaluator_id(&self) -> EvaluatorId<'_> {
        EvaluatorId::Type(TypeId::of::<Self>())
    }

    fn create_evaluator(&self) -> Box<dyn AnimationCurveEvaluator> {
        Box::new(Evaluator {
            base: self.base.create_evaluator(),
            property: Box::new(animated_field!(Transform::rotation)),
            last_node: None,
        })
    }

    fn apply(
        &self,
        curve_evaluator: &mut dyn AnimationCurveEvaluator,
        t: f32,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        let Some(curve_evaluator) = curve_evaluator.downcast_mut::<Evaluator>() else {
            let ty = TypeId::of::<Evaluator>();
            return Err(AnimationEvaluationError::InconsistentEvaluatorImplementation(ty));
        };
        curve_evaluator.last_node = Some(graph_node);
        self.base
            .apply(&mut *curve_evaluator.base, t, weight, graph_node)
    }
}

struct Evaluator {
    base: Box<dyn AnimationCurveEvaluator>,
    property: Box<dyn AnimatableProperty<Property = Quat>>,
    last_node: Option<AnimationNodeIndex>,
}

impl AnimationCurveEvaluator for Evaluator {
    fn blend(
        &mut self,
        graph_node: AnimationNodeIndex,
    ) -> std::result::Result<(), AnimationEvaluationError> {
        self.base.blend(graph_node)
    }

    fn add(
        &mut self,
        graph_node: AnimationNodeIndex,
    ) -> std::result::Result<(), AnimationEvaluationError> {
        self.base.add(graph_node)
    }

    fn push_blend_register(
        &mut self,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> std::result::Result<(), AnimationEvaluationError> {
        self.base.push_blend_register(weight, graph_node)
    }

    fn commit(
        &mut self,
        mut entity: AnimationEntityMut,
    ) -> std::result::Result<(), AnimationEvaluationError> {
        self.base.commit(entity.reborrow())?;

        let Some(node_index) = self.last_node.take() else {
            return Ok(());
        };
        let Some(table) = entity.get::<RetargetRotationTable>() else {
            return Ok(());
        };
        let Some(transformation) = table.0.get(&node_index).cloned() else {
            return Ok(());
        };
        let rotate = self.property.get_mut(&mut entity)?;
        *rotate = transformation.transform(*rotate);
        Ok(())
    }
}
