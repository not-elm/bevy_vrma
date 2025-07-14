use crate::prelude::*;
use crate::vrm::humanoid_bone::HumanoidBoneRegistry;
use bevy::animation::{
    AnimationEntityMut, AnimationEvaluationError, AnimationTarget, animated_field,
};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::any::TypeId;
use std::fmt::{Debug, Formatter};

#[derive(Clone, Debug)]
pub(crate) struct BoneRotateTransformations(HashMap<Entity, Transformation>);

impl BoneRotateTransformations {
    pub fn new(
        vrma: Entity,
        root_bone: Entity,
        registry: &HumanoidBoneRegistry,
        searcher: &ChildSearcher,
        bones: &Query<(
            &BoneRestTransform,
            &BoneRestGlobalTransform,
            &AnimationTarget,
        )>,
    ) -> Self {
        let mut transformations = HashMap::new();
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
            transformations.insert(rig_bone_entity, transformation);
        }
        Self(transformations)
    }
}

#[derive(Debug, Copy, Clone, Reflect)]
struct Transformation {
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

pub(crate) struct BoneRotationAnimationCurve {
    base: Box<dyn AnimationCurve>,
    transformations: BoneRotateTransformations,
}

impl BoneRotationAnimationCurve {
    pub fn new(
        base: VariableCurve,
        transformations: BoneRotateTransformations,
    ) -> Self {
        Self {
            base: base.0,
            transformations,
        }
    }
}

impl Debug for BoneRotationAnimationCurve {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RetargetBoneAnimationCurve")
            .field("transformations", &self.transformations)
            .finish()
    }
}

impl AnimationCurve for BoneRotationAnimationCurve {
    fn clone_value(&self) -> Box<dyn AnimationCurve> {
        Box::new(Self {
            base: self.base.clone_value(),
            transformations: self.transformations.clone(),
        })
    }

    fn domain(&self) -> Interval {
        self.base.domain()
    }

    fn evaluator_id(&self) -> EvaluatorId {
        EvaluatorId::Type(TypeId::of::<BoneRotateTransformations>())
    }

    fn create_evaluator(&self) -> Box<dyn AnimationCurveEvaluator> {
        Box::new(Evaluator {
            base: self.base.create_evaluator(),
            property: Box::new(animated_field!(Transform::rotation)),
            transformations: self.transformations.clone(),
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
        curve_evaluator
            .transformations
            .0
            .extend(&self.transformations.0);
        self.base
            .apply(&mut *curve_evaluator.base, t, weight, graph_node)?;
        //FIXME: Currently, blending multiple VRMAs with different initial poses results in incorrect interpolation.
        // To fix this, we need to implement the following at this timing, but we cannot do it due to access scope issues.
        // let curve_evaluator = curve_evaluator
        //     .downcast_mut::<AnimatableCurveEvaluator<Quat>>()
        //     .unwrap();
        // let e = curve_evaluator
        //     .evaluator
        //     .stack
        //     .pop()
        //     .unwrap();
        // curve_evaluator.evaluator.stack.push(BasicAnimationCurveEvaluatorStackElement{
        //     value: self.transformations.0.get(graph_node).unwrap().transform(e.value),
        //     weight,
        //     graph_node,
        // });
        Ok(())
    }
}

struct Evaluator {
    base: Box<dyn AnimationCurveEvaluator>,
    property: Box<dyn AnimatableProperty<Property = Quat>>,
    transformations: BoneRotateTransformations,
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
        let id = entity.id();
        let Some(transformation) = self.transformations.0.get(&id) else {
            let ty = TypeId::of::<Transformation>();
            return Err(AnimationEvaluationError::PropertyNotPresent(ty));
        };
        self.base.commit(entity.reborrow())?;
        let property = self.property.get_mut(&mut entity)?;
        *property = transformation.transform(*property);
        Ok(())
    }
}
