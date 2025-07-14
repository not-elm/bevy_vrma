use bevy::animation::{AnimationEntityMut, AnimationEvaluationError, animated_field};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::any::TypeId;
use std::fmt::{Debug, Formatter};

pub(crate) struct HipsTranslationAnimationCurve {
    base: Box<dyn AnimationCurve>,
    hips: Entity,
    transformation: Transformation,
}

impl HipsTranslationAnimationCurve {
    pub fn new(
        base: VariableCurve,
        hips: Entity,
        src_rest_g: Vec3,
        dist_rest_g: Vec3,
    ) -> Self {
        Self {
            base: base.0,
            hips,
            transformation: Transformation {
                src_rest_g,
                dist_rest_g,
            },
        }
    }
}

impl Debug for HipsTranslationAnimationCurve {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RetargetBoneTranslationAnimationCurve")
            .field("transformation", &self.transformation)
            .finish()
    }
}

impl AnimationCurve for HipsTranslationAnimationCurve {
    fn clone_value(&self) -> Box<dyn AnimationCurve> {
        Box::new(Self {
            base: self.base.clone_value(),
            hips: self.hips,
            transformation: self.transformation,
        })
    }

    #[inline]
    fn domain(&self) -> Interval {
        self.base.domain()
    }

    #[inline]
    fn evaluator_id(&self) -> EvaluatorId {
        EvaluatorId::Type(TypeId::of::<RetargetEvaluator>())
    }

    fn create_evaluator(&self) -> Box<dyn AnimationCurveEvaluator> {
        Box::new(RetargetEvaluator {
            base: self.base.create_evaluator(),
            property: Box::new(animated_field!(Transform::translation)),
            transformations: HashMap::new(),
        })
    }

    fn apply(
        &self,
        curve_evaluator: &mut dyn AnimationCurveEvaluator,
        t: f32,
        weight: f32,
        graph_node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        let Some(curve_evaluator) = curve_evaluator.downcast_mut::<RetargetEvaluator>() else {
            let ty = TypeId::of::<RetargetEvaluator>();
            return Err(AnimationEvaluationError::InconsistentEvaluatorImplementation(ty));
        };
        curve_evaluator
            .transformations
            .entry(self.hips)
            .or_insert(self.transformation);
        self.base
            .apply(&mut *curve_evaluator.base, t, weight, graph_node)?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Reflect)]
struct Transformation {
    src_rest_g: Vec3,
    dist_rest_g: Vec3,
}

impl Transformation {
    pub fn transform(
        &self,
        src_pose: Vec3,
    ) -> Vec3 {
        calc_hips_position(self.src_rest_g, src_pose, self.dist_rest_g)
    }
}

struct RetargetEvaluator {
    base: Box<dyn AnimationCurveEvaluator>,
    property: Box<dyn AnimatableProperty<Property = Vec3>>,
    transformations: HashMap<Entity, Transformation>,
}

impl AnimationCurveEvaluator for RetargetEvaluator {
    #[inline]
    fn blend(
        &mut self,
        graph_node: AnimationNodeIndex,
    ) -> std::result::Result<(), AnimationEvaluationError> {
        self.base.blend(graph_node)
    }

    #[inline]
    fn add(
        &mut self,
        graph_node: AnimationNodeIndex,
    ) -> std::result::Result<(), AnimationEvaluationError> {
        self.base.add(graph_node)
    }

    #[inline]
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
        let id = entity.id();
        let Some(transformation) = self.transformations.get(&id) else {
            let ty = TypeId::of::<Transformation>();
            return Err(AnimationEvaluationError::PropertyNotPresent(ty));
        };
        self.base.commit(entity.reborrow())?;
        let hips_pos = self.property.get_mut(&mut entity)?;
        *hips_pos = transformation.transform(*hips_pos);
        Ok(())
    }
}

#[inline]
fn calc_hips_position(
    source_rest_global_pos: Vec3,
    source_pose_pos: Vec3,
    dist_rest_global_pos: Vec3,
) -> Vec3 {
    let delta = calc_delta(source_pose_pos, source_rest_global_pos);
    let scaling = calc_scaling(dist_rest_global_pos, source_rest_global_pos);
    dist_rest_global_pos + delta * scaling
}

#[inline]
fn calc_scaling(
    dist_rest_global_pos: Vec3,
    source_rest_global_pos: Vec3,
) -> f32 {
    dist_rest_global_pos.y / source_rest_global_pos.y
}

#[inline]
fn calc_delta(
    source_pose_pos: Vec3,
    source_rest_global_pos: Vec3,
) -> Vec3 {
    source_pose_pos - source_rest_global_pos
}

#[cfg(test)]
mod tests {
    use crate::vrma::animation::bone_translation::{calc_delta, calc_scaling};
    use bevy::math::Vec3;

    #[test]
    fn test_scaling() {
        let scaling = calc_scaling(Vec3::splat(1.), Vec3::splat(2.));
        assert!((scaling - 0.5) < 0.001);
    }

    #[test]
    fn test_delta() {
        let delta = calc_delta(Vec3::splat(1.), Vec3::splat(2.));
        assert_eq!(delta, Vec3::splat(-1.));
    }
}
