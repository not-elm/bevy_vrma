use crate::prelude::{BoneRestGlobalTransform, BoneRestTransform, ChildSearcher};
use crate::vrm::expressions::VrmExpressionRegistry;
use crate::vrm::humanoid_bone::HumanoidBoneRegistry;
use crate::vrma::animation::bone_rotation::{
    BoneRotationAnimationCurve, register_rotate_transformation,
};
use crate::vrma::animation::bone_translation::{
    HipsTranslationAnimationCurve, register_hips_translation_transformation,
};
use crate::vrma::{VrmAnimationClipHandle, VrmAnimationNodeIndex};
use bevy::animation::{AnimationTarget, animated_field};
use bevy::app::App;
use bevy::prelude::*;

#[derive(Event)]
pub(crate) struct RequestUpdateAnimationGraph {
    pub(crate) vrm: Entity,
    pub(crate) vrma: Entity,
}

#[derive(Event)]
struct RequestUpdateAnimationClips;

pub(super) struct VrmaAnimationGraphPlugin;

impl Plugin for VrmaAnimationGraphPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.add_observer(apply_animation_graph)
            .add_observer(apply_replace_humanoid_bone_animation_clips)
            .add_observer(apply_regenerate_expression_clips);
    }
}

fn apply_animation_graph(
    trigger: Trigger<RequestUpdateAnimationGraph>,
    mut commands: Commands,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    childrens: Query<&Children>,
    vrmas: Query<(Entity, &VrmAnimationClipHandle)>,
    child_searcher: ChildSearcher,
    entities: Query<(Has<AnimationPlayer>, Option<&AnimationGraphHandle>)>,
) {
    let vrma_entity = trigger.vrma;
    let vrm_entity = trigger.vrm;
    let Ok(children) = childrens.get(vrm_entity) else {
        return;
    };
    let animation_graph = generate_animation_graph(&mut commands, &vrmas, children);
    let animation_graph_handle = AnimationGraphHandle(graphs.add(animation_graph));
    insert_animation_graph_into_root_bone(
        vrm_entity,
        animation_graph_handle.clone(),
        &mut commands,
        &child_searcher,
    );
    insert_animation_graph_into_expressions(
        trigger.vrm,
        &mut commands,
        &mut graphs,
        &animation_graph_handle,
        &entities,
        &child_searcher,
        &childrens,
    );
    commands
        .entity(vrma_entity)
        .trigger(RequestUpdateAnimationClips);
}

fn generate_animation_graph(
    commands: &mut Commands,
    vrmas_query: &Query<(Entity, &VrmAnimationClipHandle)>,
    children: &Children,
) -> AnimationGraph {
    let vrmas = children
        .iter()
        .flat_map(|child| vrmas_query.get(child).ok())
        .collect::<Vec<_>>();
    let (graph, nodes) = AnimationGraph::from_clips(vrmas.iter().map(|(_, h)| h.0.clone()));
    for (i, (entity, _)) in vrmas.iter().enumerate() {
        commands
            .entity(*entity)
            .insert(VrmAnimationNodeIndex(nodes[i]));
    }
    graph
}

fn insert_animation_graph_into_root_bone(
    vrm: Entity,
    animation_graph_handle: AnimationGraphHandle,
    commands: &mut Commands,
    searcher: &ChildSearcher,
) {
    let Some(root_bone) = searcher.find_root_bone(vrm) else {
        return;
    };
    commands.entity(root_bone).insert(animation_graph_handle);
}

fn insert_animation_graph_into_expressions(
    entity: Entity,
    commands: &mut Commands,
    graphs: &mut Assets<AnimationGraph>,
    animation_graph_handle: &AnimationGraphHandle,
    expressions: &Query<(Has<AnimationPlayer>, Option<&AnimationGraphHandle>)>,
    searcher: &ChildSearcher,
    childrens: &Query<&Children>,
) {
    let Some(expressions_root) = searcher.find_expressions_root(entity) else {
        return;
    };
    let Ok(expression_children) = childrens.get(expressions_root) else {
        return;
    };
    for expression in expression_children.iter() {
        let Ok((has_player, previous_handle)) = expressions.get(expression) else {
            return;
        };
        if let Some(previous_handle) = previous_handle {
            graphs.remove(previous_handle);
        }
        if has_player {
            commands
                .entity(expression)
                .insert(animation_graph_handle.clone());
        }
    }
}

fn apply_replace_humanoid_bone_animation_clips(
    trigger: Trigger<RequestUpdateAnimationClips>,
    mut clips: ResMut<Assets<AnimationClip>>,
    clip_handles: Query<&VrmAnimationClipHandle>,
    parents: Query<&ChildOf>,
    vrms: Query<&HumanoidBoneRegistry>,
    bones: Query<(
        &BoneRestTransform,
        &BoneRestGlobalTransform,
        &AnimationTarget,
    )>,
    nodes: Query<&VrmAnimationNodeIndex>,
    searcher: ChildSearcher,
) {
    let vrma_entity = trigger.target();
    let Ok(ChildOf(vrm_entity)) = parents.get(vrma_entity) else {
        return;
    };
    let Ok(vrma_node_index) = nodes.get(vrma_entity) else {
        return;
    };
    let Ok(registry) = vrms.get(vrma_entity) else {
        return;
    };
    let Ok(vrm_animation_clip_handle) = clip_handles.get(vrma_entity) else {
        return;
    };
    let Some(root_bone) = searcher.find_root_bone(*vrm_entity) else {
        return;
    };
    let Some(clip) = clips.get_mut(vrm_animation_clip_handle.0.id()) else {
        return;
    };
    register_rotate_transformation(
        vrma_entity,
        vrma_node_index.0,
        root_bone,
        registry,
        &searcher,
        &bones,
    );
    replace_bone_animation_clips(
        clip,
        vrma_node_index.0,
        vrma_entity,
        root_bone,
        registry,
        &searcher,
        &bones,
    );
}

fn replace_bone_animation_clips(
    clip: &mut AnimationClip,
    node_index: AnimationNodeIndex,
    vrma_entity: Entity,
    root_bone: Entity,
    registry: &HumanoidBoneRegistry,
    searcher: &ChildSearcher,
    bones: &Query<(
        &BoneRestTransform,
        &BoneRestGlobalTransform,
        &AnimationTarget,
    )>,
) {
    let animation_curves = clip.curves_mut();
    for (bone, name) in registry.iter() {
        let Some(vrma_bone_entity) = searcher.find_from_name(vrma_entity, name) else {
            continue;
        };
        let Some(bone_entity) = searcher.find_by_bone_name(root_bone, bone) else {
            continue;
        };
        let Ok((_, src_rest_gtf, vrma_bone_target)) = bones.get(vrma_bone_entity) else {
            continue;
        };
        let Ok((_, dist_rest_gtf, bone_target)) = bones.get(bone_entity) else {
            continue;
        };
        if bone.as_str() == "hips" {
            register_hips_translation_transformation(
                node_index,
                bone_entity,
                src_rest_gtf,
                dist_rest_gtf,
            );
        }
        if let Some(curves) = animation_curves.remove(&vrma_bone_target.id) {
            let mut cs = Vec::new();
            for c in curves.iter() {
                cs.push(animation_curve(c.clone(), bone.as_str() == "hips"));
            }
            animation_curves.insert(bone_target.id, cs.clone());
        }
    }
}

fn animation_curve(
    original: VariableCurve,
    hips: bool,
) -> VariableCurve {
    let EvaluatorId::ComponentField(target_component) = original.0.evaluator_id() else {
        return original;
    };

    let translation_field = animated_field!(Transform::translation);
    let EvaluatorId::ComponentField(translation_component) = translation_field.evaluator_id()
    else {
        return original;
    };

    let rotation_field = animated_field!(Transform::rotation);
    let EvaluatorId::ComponentField(rotation_component) = rotation_field.evaluator_id() else {
        return original;
    };

    if target_component == rotation_component {
        VariableCurve(Box::new(BoneRotationAnimationCurve { base: original.0 }))
    } else if hips && target_component == translation_component {
        VariableCurve(Box::new(HipsTranslationAnimationCurve { base: original.0 }))
    } else {
        original
    }
}

fn apply_regenerate_expression_clips(
    trigger: Trigger<RequestUpdateAnimationClips>,
    mut clips: ResMut<Assets<AnimationClip>>,
    clip_handles: Query<&VrmAnimationClipHandle>,
    animation_targets: Query<&AnimationTarget>,
    expressions: Query<&VrmExpressionRegistry>,
    searcher: ChildSearcher,
    parents: Query<&ChildOf>,
) {
    let vrma_entity = trigger.target();
    let Ok(vrm_entity) = parents.get(vrma_entity).map(|c| c.parent()) else {
        return;
    };
    let Some(expressions_root) = searcher.find_expressions_root(vrm_entity) else {
        return;
    };
    let Ok(vrm_animation_clip_handle) = clip_handles.get(vrma_entity) else {
        return;
    };
    let Some(clip) = clips.get_mut(vrm_animation_clip_handle.0.id()) else {
        return;
    };
    let Ok(registry) = expressions.get(vrm_entity) else {
        return;
    };
    for (expression, _) in registry.iter() {
        let Some(vrma_expression) = searcher.find_from_name(vrma_entity, expression) else {
            continue;
        };
        let Some(expression_entity) = searcher.find_from_name(expressions_root, expression) else {
            continue;
        };
        let Ok(vrma_target) = animation_targets.get(vrma_expression) else {
            return;
        };
        let Ok(target) = animation_targets.get(expression_entity) else {
            return;
        };
        let animation_curves = clip.curves_mut();
        if let Some(curves) = animation_curves.remove(&vrma_target.id) {
            animation_curves.insert(target.id, curves);
        }
    }
}

#[cfg(test)]
mod tests {}
