use crate::prelude::{RestGlobalTransform, RestTransform};
use bevy::animation::{AnimationEntityMut, AnimationEvaluationError, animated_field};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::any::TypeId;
use std::fmt::{Debug, Formatter};
use std::sync::Mutex;

pub fn register_hips_translation_transformation(
    node_index: AnimationNodeIndex,
    hips: Entity,
    src_rest: &RestTransform,
    src_rest_g: &RestGlobalTransform,
    dist_rest: &RestTransform,
    dist_rest_g: &RestGlobalTransform,
) {
    let transformations = Transformation {
        src_rest_local: src_rest.translation,
        src_rest_g: src_rest_g.translation(),
        dist_rest_local: dist_rest.translation,
        dist_rest_g: dist_rest_g.translation(),
    };
    HIPS_TRANSFORMATIONS
        .lock()
        .expect("Failed to lock HIPS_TRANSFORMATIONS")
        .insert((hips, node_index), transformations);
}

static HIPS_TRANSFORMATIONS: Mutex<HashMap<(Entity, AnimationNodeIndex), Transformation>> =
    Mutex::new(HashMap::new());

pub(crate) struct HipsTranslationAnimationCurve {
    pub base: Box<dyn AnimationCurve>,
}

impl Debug for HipsTranslationAnimationCurve {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RetargetBoneTranslationAnimationCurve")
            .finish()
    }
}

impl AnimationCurve for HipsTranslationAnimationCurve {
    fn clone_value(&self) -> Box<dyn AnimationCurve> {
        Box::new(Self {
            base: self.base.clone_value(),
        })
    }

    #[inline]
    fn domain(&self) -> Interval {
        self.base.domain()
    }

    #[inline]
    fn evaluator_id(&self) -> EvaluatorId<'_> {
        EvaluatorId::Type(TypeId::of::<RetargetEvaluator>())
    }

    fn create_evaluator(&self) -> Box<dyn AnimationCurveEvaluator> {
        Box::new(RetargetEvaluator {
            base: self.base.create_evaluator(),
            property: Box::new(animated_field!(Transform::translation)),
            nodes: Vec::new(),
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
        curve_evaluator.nodes.push(graph_node);
        self.base
            .apply(&mut *curve_evaluator.base, t, weight, graph_node)?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Reflect)]
struct Transformation {
    src_rest_local: Vec3,
    src_rest_g: Vec3,
    dist_rest_local: Vec3,
    dist_rest_g: Vec3,
}

impl Transformation {
    pub fn transform(
        &self,
        src_pose: Vec3,
    ) -> Vec3 {
        calc_hips_position(
            self.src_rest_local,
            self.src_rest_g,
            src_pose,
            self.dist_rest_local,
            self.dist_rest_g,
        )
    }
}

struct RetargetEvaluator {
    base: Box<dyn AnimationCurveEvaluator>,
    property: Box<dyn AnimatableProperty<Property = Vec3>>,
    nodes: Vec<AnimationNodeIndex>,
    transformations: HashMap<(Entity, AnimationNodeIndex), Transformation>,
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

    #[inline]
    fn commit(
        &mut self,
        mut entity: AnimationEntityMut,
    ) -> std::result::Result<(), AnimationEvaluationError> {
        let hips_bone = entity.id();
        let node = self.nodes.pop().unwrap();
        let transformation = self
            .transformations
            .entry((hips_bone, node))
            .or_insert_with(|| {
                let hips_transformations = HIPS_TRANSFORMATIONS
                    .lock()
                    .expect("Failed to lock HIPS_TRANSFORMATIONS");
                hips_transformations
                    .get(&(hips_bone, node))
                    .cloned()
                    .unwrap()
            });
        self.base.commit(entity.reborrow())?;
        let hips_pos = self.property.get_mut(&mut entity)?;
        *hips_pos = transformation.transform(*hips_pos);
        Ok(())
    }
}

/// Retargets a hips bone translation from source to target model space.
///
/// Uses **local** rest positions for delta computation and result placement
/// (matching `Transform::translation` coordinate space), and **global** rest
/// positions only for the Y-based height scaling ratio.
#[inline]
fn calc_hips_position(
    src_rest_local: Vec3,
    src_rest_global: Vec3,
    src_pose: Vec3,
    dst_rest_local: Vec3,
    dst_rest_global: Vec3,
) -> Vec3 {
    let delta = src_pose - src_rest_local;
    let scaling = calc_scaling(dst_rest_global, src_rest_global);
    dst_rest_local + delta * scaling
}

#[inline]
fn calc_scaling(
    dist_rest_global_pos: Vec3,
    source_rest_global_pos: Vec3,
) -> f32 {
    dist_rest_global_pos.y / source_rest_global_pos.y
}

#[cfg(test)]
mod tests {
    use crate::vrma::animation::bone_translation::{calc_hips_position, calc_scaling};
    use bevy::math::Vec3;

    #[test]
    fn test_scaling() {
        let scaling = calc_scaling(Vec3::splat(1.), Vec3::splat(2.));
        assert!((scaling - 0.5) < 0.001);
    }

    #[test]
    fn test_y_only_animation_no_x_shift() {
        // Source model: hips local rest at (0, 0.9, 0.01), global at (0.02, 0.9, 0.01)
        // Target model: hips local rest at (0, 1.0, 0.01), global at (0.01, 1.0, 0.01)
        // Animation: only Y changes (0, 0.95, 0.01) — no X movement
        let result = calc_hips_position(
            Vec3::new(0.0, 0.9, 0.01),  // src_rest_local
            Vec3::new(0.02, 0.9, 0.01), // src_rest_global
            Vec3::new(0.0, 0.95, 0.01), // src_pose (only Y changed)
            Vec3::new(0.0, 1.0, 0.01),  // dst_rest_local
            Vec3::new(0.01, 1.0, 0.01), // dst_rest_global
        );
        // X should remain at dst_rest_local.x (no phantom shift)
        assert!(
            (result.x - 0.0).abs() < 0.001,
            "X should not shift: {}",
            result.x
        );
        // Z should remain at dst_rest_local.z
        assert!(
            (result.z - 0.01).abs() < 0.001,
            "Z should not shift: {}",
            result.z
        );
    }

    #[test]
    fn test_local_equals_global_no_regression() {
        // When local == global (hips is root bone), result should be the same as before.
        let src_rest = Vec3::new(0.0, 0.9, 0.0);
        let dst_rest = Vec3::new(0.0, 1.0, 0.0);
        let src_pose = Vec3::new(0.0, 0.95, 0.0);
        let result = calc_hips_position(src_rest, src_rest, src_pose, dst_rest, dst_rest);
        let scaling = dst_rest.y / src_rest.y;
        let expected = dst_rest + (src_pose - src_rest) * scaling;
        assert!((result - expected).length() < 0.001);
    }
}
