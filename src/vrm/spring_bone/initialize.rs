use crate::prelude::ChildSearcher;
use crate::vrm::humanoid_bone::RequestInitializeHumanoidBones;
use crate::vrm::spring_bone::registry::{
    SpringColliderRegistry, SpringJointPropsRegistry, SpringNodeRegistry,
};
use crate::vrm::spring_bone::{
    SpringCenterNode, SpringColliders, SpringJointState, SpringJoints, SpringRoot,
};
use bevy::app::{App, Update};
use bevy::prelude::*;

#[derive(Event)]
pub(crate) struct RequestInitializeSpringBone;

pub struct SpringBoneInitializePlugin;

impl Plugin for SpringBoneInitializePlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.add_systems(Update, init_spring_joint_states)
            .add_observer(apply_initialize_joint_props)
            .add_observer(apply_initialize_collider_shapes)
            .add_observer(apply_initialize_spring_roots);
    }
}

fn apply_initialize_joint_props(
    trigger: Trigger<RequestInitializeHumanoidBones>,
    mut commands: Commands,
    child_searcher: ChildSearcher,
    models: Query<&SpringJointPropsRegistry>,
) {
    let root = trigger.target();
    let Ok(nodes) = models.get(root) else {
        return;
    };
    for (name, props) in nodes.iter() {
        let Some(joint_entity) = child_searcher.find_from_name(root, name.as_str()) else {
            continue;
        };
        commands.entity(joint_entity).insert(*props);
    }
}

fn apply_initialize_collider_shapes(
    trigger: Trigger<RequestInitializeSpringBone>,
    mut commands: Commands,
    child_searcher: ChildSearcher,
    models: Query<&SpringColliderRegistry>,
) {
    let entity = trigger.target();
    let Ok(registry) = models.get(entity) else {
        return;
    };
    for (name, shape) in registry.iter() {
        let Some(collider_entity) = child_searcher.find_from_name(entity, name) else {
            continue;
        };
        commands.entity(collider_entity).insert(*shape);
    }
}

fn apply_initialize_spring_roots(
    trigger: Trigger<RequestInitializeSpringBone>,
    mut commands: Commands,
    child_searcher: ChildSearcher,
    models: Query<&SpringNodeRegistry>,
) {
    let entity = trigger.target();
    let Ok(registry) = models.get(entity) else {
        return;
    };
    for spring_root in registry.0.iter().map(|spring| SpringRoot {
        center_node: SpringCenterNode(
            spring
                .center
                .as_ref()
                .and_then(|center| child_searcher.find_from_name(entity, center.as_str())),
        ),
        joints: SpringJoints(
            spring
                .joints
                .iter()
                .filter_map(|joint| child_searcher.find_from_name(entity, joint.as_str()))
                .collect(),
        ),
        colliders: SpringColliders(
            spring
                .colliders
                .iter()
                .filter_map(|(collider, shape)| {
                    let name = child_searcher.find_from_name(entity, collider.as_str())?;
                    Some((name, *shape))
                })
                .collect(),
        ),
    }) {
        let Some(root) = spring_root.joints.first() else {
            continue;
        };
        commands.entity(*root).insert(spring_root);
    }
}

fn init_spring_joint_states(
    par_commands: ParallelCommands,
    spring_roots: Query<&SpringRoot, Added<SpringRoot>>,
    joints: Query<&Transform>,
    global_transforms: Query<&GlobalTransform>,
) {
    spring_roots.par_iter().for_each(|root| {
        for w in root.joints.windows(2) {
            let head_entity = w[0];
            let joint_entity = w[1];
            let Ok(head_tf) = joints.get(head_entity) else {
                continue;
            };
            let Ok(tail_tf) = joints.get(joint_entity) else {
                continue;
            };
            let Ok(tail_gtf) = global_transforms.get(joint_entity) else {
                continue;
            };
            let tail_pos = root
                .center_node
                .and_then(|center| global_transforms.get(center).ok())
                .map(|center_gtf| tail_gtf.reparented_to(center_gtf).translation)
                .unwrap_or(tail_gtf.translation());
            let state = SpringJointState {
                prev_tail: tail_pos,
                current_tail: tail_pos,
                bone_axis: tail_tf.translation.normalize(),
                bone_length: tail_tf.translation.length(),
                initial_local_matrix: head_tf.compute_matrix(),
                initial_local_rotation: head_tf.rotation,
            };
            par_commands.command_scope(|mut commands| {
                commands.entity(head_entity).insert(state);
            });
        }
    });
}

#[cfg(test)]
mod tests {}
