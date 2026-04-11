use crate::prelude::{ChildSearcher, RestGlobalTransform, RestTransform};
use crate::vrm::expressions::VrmExpressionRegistry;
use crate::vrm::humanoid_bone::HumanoidBoneRegistry;
use crate::vrma::animation::bake::{bake_rotation_curve, bake_translation_curve};
use crate::vrma::animation::bone_rotation::{
    RetargetRotationTable, compute_rotation_transformations,
};
use crate::vrma::animation::bone_translation::{
    RetargetTranslationTable, compute_hips_transformation,
};
use crate::vrma::{VrmAnimationClipHandle, VrmAnimationNodeIndex};
use bevy::animation::{AnimationTargetId, animated_field};
use bevy::app::App;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Marker: VRMA clip needs baking.
#[derive(Component)]
pub(crate) struct NeedsBake;

#[derive(Event)]
pub(crate) struct RequestUpdateAnimationGraph {
    pub(crate) vrm: Entity,
    pub(crate) vrma: Entity,
}

#[derive(EntityEvent)]
struct RequestUpdateAnimationClips(Entity);

pub(super) struct VrmaAnimationGraphPlugin;

impl Plugin for VrmaAnimationGraphPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.add_observer(apply_animation_graph)
            .add_observer(apply_replace_humanoid_bone_animation_clips)
            .add_observer(apply_regenerate_expression_clips)
            .add_systems(Update, apply_bake_clips);
    }
}

fn apply_animation_graph(
    trigger: On<RequestUpdateAnimationGraph>,
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
            continue;
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
    trigger: On<RequestUpdateAnimationClips>,
    mut commands: Commands,
    mut clips: ResMut<Assets<AnimationClip>>,
    clip_handles: Query<&VrmAnimationClipHandle>,
    parents: Query<&ChildOf>,
    vrms: Query<&HumanoidBoneRegistry>,
    bones: Query<(&RestTransform, &RestGlobalTransform, &AnimationTargetId)>,
    nodes: Query<&VrmAnimationNodeIndex>,
    searcher: ChildSearcher,
) {
    let vrma_entity = trigger.event_target();
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
    // Note: AnimationClip is already cloned per-VRMA in initialize.rs:67-71
    let Some(clip) = clips.get_mut(vrm_animation_clip_handle.0.id()) else {
        return;
    };
    let transformations = compute_rotation_transformations(
        vrma_entity,
        vrma_node_index.0,
        root_bone,
        registry,
        &searcher,
        &bones,
    );
    for (bone_entity, node_index, transformation) in transformations {
        commands
            .entity(bone_entity)
            .entry::<RetargetRotationTable>()
            .and_modify(move |mut table| {
                table.0.insert(node_index, transformation);
            })
            .or_insert(RetargetRotationTable(HashMap::from([(
                node_index,
                transformation,
            )])));
    }
    replace_bone_animation_clips(
        &mut commands,
        clip,
        vrma_node_index.0,
        vrma_entity,
        root_bone,
        registry,
        &searcher,
        &bones,
    );
    commands.entity(vrma_entity).insert(NeedsBake);
}

fn replace_bone_animation_clips(
    commands: &mut Commands,
    clip: &mut AnimationClip,
    node_index: AnimationNodeIndex,
    vrma_entity: Entity,
    root_bone: Entity,
    registry: &HumanoidBoneRegistry,
    searcher: &ChildSearcher,
    bones: &Query<(&RestTransform, &RestGlobalTransform, &AnimationTargetId)>,
) {
    let animation_curves = clip.curves_mut();
    for (bone, name) in registry.iter() {
        let Some(vrma_bone_entity) = searcher.find_from_name(vrma_entity, name) else {
            continue;
        };
        let Some(bone_entity) = searcher.find_by_bone_name(root_bone, bone) else {
            continue;
        };
        let Ok((src_rest_tf, src_rest_gtf, vrma_bone_target)) = bones.get(vrma_bone_entity) else {
            continue;
        };
        let Ok((dist_rest_tf, dist_rest_gtf, bone_target)) = bones.get(bone_entity) else {
            continue;
        };
        if bone.as_str() == "hips" {
            let (node_idx, hips_tf) = compute_hips_transformation(
                node_index,
                src_rest_tf,
                src_rest_gtf,
                dist_rest_tf,
                dist_rest_gtf,
            );
            commands
                .entity(bone_entity)
                .entry::<RetargetTranslationTable>()
                .and_modify(move |mut table| {
                    table.0.insert(node_idx, hips_tf);
                })
                .or_insert(RetargetTranslationTable(HashMap::from([(
                    node_idx, hips_tf,
                )])));
        }
        if let Some(curves) = animation_curves.remove(vrma_bone_target) {
            animation_curves.insert(*bone_target, curves);
        }
    }
}

fn apply_bake_clips(world: &mut World) {
    // Collect VRMA entities that need baking, including their AnimationNodeIndex
    let mut to_bake: Vec<(Entity, Handle<AnimationClip>, AnimationNodeIndex)> = Vec::new();
    {
        let mut query = world.query_filtered::<(
            Entity,
            &VrmAnimationClipHandle,
            &VrmAnimationNodeIndex,
        ), With<NeedsBake>>();
        for (entity, clip_handle, node_index) in query.iter(world) {
            to_bake.push((entity, clip_handle.0.clone(), node_index.0));
        }
    }
    if to_bake.is_empty() {
        return;
    }

    let rotation_field = animated_field!(Transform::rotation);
    let EvaluatorId::ComponentField(rotation_component) = rotation_field.evaluator_id() else {
        return;
    };
    let rotation_component = *rotation_component;
    let translation_field = animated_field!(Transform::translation);
    let EvaluatorId::ComponentField(translation_component) = translation_field.evaluator_id()
    else {
        return;
    };
    let translation_component = *translation_component;

    for (vrma_entity, clip_handle, node_index) in to_bake {
        // Get the clip data first; only remove NeedsBake after successful fetch
        let Some(clip) = world
            .resource::<Assets<AnimationClip>>()
            .get(clip_handle.id())
            .cloned()
        else {
            continue;
        };
        world.entity_mut(vrma_entity).remove::<NeedsBake>();

        let mut new_curves: HashMap<AnimationTargetId, Vec<VariableCurve>> = HashMap::new();

        for (target_id, variable_curves) in clip.curves().iter() {
            // Find the VRM bone entity that has this AnimationTargetId.
            // Filter to entities with RetargetRotationTable to avoid matching VRMA bone entities.
            let bone_entity = {
                let mut q = world
                    .query_filtered::<(Entity, &AnimationTargetId), With<RetargetRotationTable>>();
                q.iter(world)
                    .find(|(_, tid)| **tid == *target_id)
                    .map(|(e, _)| e)
            };

            let mut baked_curves = Vec::new();
            for vc in variable_curves.iter() {
                let mut baked = false;

                if let Some(bone) = bone_entity
                    && let EvaluatorId::ComponentField(target) = vc.0.evaluator_id()
                {
                    if *target == rotation_component {
                        // Use the specific VRMA's AnimationNodeIndex to get the correct transformation
                        if let Some(table) = world.get::<RetargetRotationTable>(bone)
                            && let Some(transformation) = table.0.get(&node_index).cloned()
                            && let Some(baked_vc) = bake_rotation_curve(vc, &transformation, world)
                        {
                            baked_curves.push(baked_vc);
                            baked = true;
                        }
                    } else if *target == translation_component
                        && let Some(table) = world.get::<RetargetTranslationTable>(bone)
                        && let Some(transformation) = table.0.get(&node_index).cloned()
                        && let Some(baked_vc) = bake_translation_curve(vc, &transformation, world)
                    {
                        baked_curves.push(baked_vc);
                        baked = true;
                    }
                }

                if !baked {
                    baked_curves.push(vc.clone());
                }
            }
            new_curves.insert(*target_id, baked_curves);
        }

        // Replace the clip's curves with baked ones
        let mut clip_assets = world.resource_mut::<Assets<AnimationClip>>();
        if let Some(clip) = clip_assets.get_mut(clip_handle.id()) {
            let curves = clip.curves_mut();
            curves.clear();
            for (target_id, variable_curves) in new_curves {
                for vc in variable_curves {
                    curves.entry(target_id).or_default().push(vc);
                }
            }
        }
    }
}

fn apply_regenerate_expression_clips(
    trigger: On<RequestUpdateAnimationClips>,
    mut clips: ResMut<Assets<AnimationClip>>,
    clip_handles: Query<&VrmAnimationClipHandle>,
    animation_targets: Query<&AnimationTargetId>,
    expressions: Query<&VrmExpressionRegistry>,
    searcher: ChildSearcher,
    parents: Query<&ChildOf>,
) {
    let vrma_entity = trigger.event_target();
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
            continue;
        };
        let Ok(target) = animation_targets.get(expression_entity) else {
            continue;
        };
        let animation_curves = clip.curves_mut();
        if let Some(curves) = animation_curves.remove(vrma_target) {
            animation_curves.insert(*target, curves);
        }
    }
}

#[cfg(test)]
mod tests {}
