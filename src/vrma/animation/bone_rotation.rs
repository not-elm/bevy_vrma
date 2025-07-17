use crate::prelude::*;
use crate::vrm::humanoid_bone::HumanoidBoneRegistry;
use bevy::animation::{
    AnimationEntityMut, AnimationEvaluationError, AnimationTarget, animated_field,
};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::any::TypeId;
use std::fmt::{Debug, Formatter};
use std::sync::Mutex;

pub(crate) fn register_rotate_transformation(
    vrma: Entity,
    node_index: AnimationNodeIndex,
    root_bone: Entity,
    registry: &HumanoidBoneRegistry,
    searcher: &ChildSearcher,
    bones: &Query<(
        &BoneRestTransform,
        &BoneRestGlobalTransform,
        &AnimationTarget,
    )>,
) {
    let transformations =
        BoneRotateTransformations::new(vrma, node_index, root_bone, registry, searcher, bones);
    BONE_ROTATION_TRANSFORMATIONS
        .lock()
        .expect("Failed to lock BONE_ROTATION_TRANSFORMATIONS")
        .extend(transformations.0);
}

static BONE_ROTATION_TRANSFORMATIONS: Mutex<HashMap<(Entity, AnimationNodeIndex), Transformation>> =
    Mutex::new(HashMap::new());

#[derive(Clone, Debug, Deref, DerefMut)]
struct BoneRotateTransformations(pub HashMap<(Entity, AnimationNodeIndex), Transformation>);

impl BoneRotateTransformations {
    pub fn new(
        vrma: Entity,
        node_index: AnimationNodeIndex,
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
            transformations.insert((rig_bone_entity, node_index), transformation);
        }
        Self(transformations)
    }
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

    fn evaluator_id(&self) -> EvaluatorId {
        EvaluatorId::Type(TypeId::of::<Self>())
    }

    fn create_evaluator(&self) -> Box<dyn AnimationCurveEvaluator> {
        Box::new(Evaluator {
            base: self.base.create_evaluator(),
            property: Box::new(animated_field!(Transform::rotation)),
            nodes: Vec::default(),
            transformations: HashMap::default(),
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
        curve_evaluator.nodes.push(graph_node);
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
    nodes: Vec<AnimationNodeIndex>,
    transformations: HashMap<(Entity, AnimationNodeIndex), Transformation>,
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
        let bone_entity = entity.id();
        self.base.commit(entity.reborrow())?;
        let node_index = self.nodes.pop().unwrap();
        let transformation = self
            .transformations
            .entry((bone_entity, node_index))
            .or_insert_with(|| {
                let t = BONE_ROTATION_TRANSFORMATIONS.lock().unwrap();
                t.get(&(bone_entity, node_index)).cloned().unwrap()
            });
        let rotate = self.property.get_mut(&mut entity)?;
        *rotate = transformation.transform(*rotate);
        Ok(())
    }
}
