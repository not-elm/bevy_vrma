use crate::macros::marker_component;
use crate::system_param::child_searcher::ChildSearcher;
use crate::vrm::humanoid_bone::HumanoidBoneRegistry;
use crate::vrm::spring_bone::registry::{
    SpringColliderRegistry, SpringJointPropsRegistry, SpringNodeRegistry,
};
use crate::vrm::spring_bone::{SpringJointState, SpringRoot};
use bevy::app::{App, Update};
use bevy::math::NormedVectorSpace;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct SpringBoneAttachPlugin;

impl Plugin for SpringBoneAttachPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<AttachedJointProps>()
            .register_type::<AttachedColliderShapes>()
            .register_type::<AttachedSpringRoots>()
            .add_systems(
                Update,
                (
                    attach_joint_props,
                    attach_collider_shapes,
                    attach_spring_roots,
                    init_spring_joint_states,
                ),
            );
    }
}

marker_component!(
    /// A marker component that indicates that initialization of joint props has been completed.
    ///
    /// This is attached to the VRM entity.
    AttachedJointProps
);

marker_component!(
    /// A marker component that indicates that initialization of collider shapes has been completed.
    ///
    /// This is attached to the VRM entity.
    AttachedColliderShapes
);

marker_component!(
    /// A marker component that indicates that initialization of spring chains has been completed.
    ///
    /// This is attached to the VRM entity.
    AttachedSpringRoots
);

fn attach_joint_props(
    par_commands: ParallelCommands,
    child_searcher: ChildSearcher,
    mascots: Query<
        (Entity, &SpringJointPropsRegistry, &HumanoidBoneRegistry),
        Without<AttachedJointProps>,
    >,
) {
    mascots
        .par_iter()
        .for_each(|(entity, nodes, bone_registry)| {
            if !child_searcher.has_been_spawned_all_bones(entity, bone_registry) {
                return;
            }

            for (name, props) in nodes.iter() {
                let Some(joint_entity) = child_searcher.find_from_name(entity, name.as_str())
                else {
                    continue;
                };
                par_commands.command_scope(|mut commands| {
                    commands.entity(joint_entity).insert(*props);
                });
            }
            par_commands.command_scope(|mut commands| {
                commands.entity(entity).insert(AttachedJointProps);
            });
        });
}

fn attach_collider_shapes(
    par_commands: ParallelCommands,
    child_searcher: ChildSearcher,
    vrm: Query<
        (Entity, &SpringColliderRegistry, &HumanoidBoneRegistry),
        Without<AttachedColliderShapes>,
    >,
) {
    vrm.par_iter().for_each(|(entity, nodes, bone_registry)| {
        if !child_searcher.has_been_spawned_all_bones(entity, bone_registry) {
            return;
        }
        for (name, shape) in nodes.iter() {
            let Some(collider_entity) = child_searcher.find_from_name(entity, name) else {
                continue;
            };
            par_commands.command_scope(|mut commands| {
                commands.entity(collider_entity).insert(*shape);
            });
        }
        par_commands.command_scope(|mut commands| {
            commands.entity(entity).insert(AttachedColliderShapes);
        });
    });
}

fn attach_spring_roots(
    par_commands: ParallelCommands,
    child_searcher: ChildSearcher,
    mascots: Query<
        (Entity, &SpringNodeRegistry, &HumanoidBoneRegistry),
        Without<AttachedSpringRoots>,
    >,
) {
    mascots
        .par_iter()
        .for_each(|(entity, registry, bone_registry)| {
            if !child_searcher.has_been_spawned_all_bones(entity, bone_registry) {
                return;
            }

            for spring_root in registry.0.iter().map(|spring| SpringRoot {
                center_node: spring
                    .center
                    .as_ref()
                    .and_then(|center| child_searcher.find_from_name(entity, center.as_str())),
                joints: spring
                    .joints
                    .iter()
                    .filter_map(|joint| child_searcher.find_from_name(entity, joint.as_str()))
                    .collect(),
                colliders: spring
                    .colliders
                    .iter()
                    .filter_map(|collider| child_searcher.find_from_name(entity, collider.as_str()))
                    .collect(),
            }) {
                let Some(root) = spring_root.joints.first() else {
                    continue;
                };
                let root = *root;
                par_commands.command_scope(|mut commands| {
                    commands.entity(root).insert(spring_root);
                });
            }
            par_commands.command_scope(|mut commands| {
                commands.entity(entity).insert(AttachedSpringRoots);
            });
        });
}

fn init_spring_joint_states(
    par_commands: ParallelCommands,
    spring_roots: Query<(Entity, &SpringRoot), Added<SpringRoot>>,
    joints: Query<&Transform>,
) {
    spring_roots.par_iter().for_each(|(root_entity, root)| {
        let mut parent = root_entity;
        for joint_entity in root.joints.iter() {
            let Ok(tf) = joints.get(*joint_entity) else {
                continue;
            };
            let state = SpringJointState {
                prev_tail: tf.translation,
                current_tail: tf.translation,
                bone_axis: tf.translation.normalize(),
                bone_length: tf.translation.norm(),
                initial_local_matrix: tf.compute_matrix(),
                initial_local_rotation: tf.rotation,
            };
            par_commands.command_scope(|mut commands| {
                commands.entity(parent).insert(state);
            });
            parent = *joint_entity;
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::success;
    use crate::tests::{test_app, TestResult};
    use crate::vrm::extensions::vrmc_spring_bone::ColliderShape;
    use crate::vrm::humanoid_bone::HumanoidBoneRegistry;
    use crate::vrm::spring_bone::attach::{
        attach_collider_shapes, attach_joint_props, attach_spring_roots, init_spring_joint_states,
        AttachedColliderShapes, AttachedJointProps, AttachedSpringRoots,
    };
    use crate::vrm::spring_bone::registry::{
        SpringColliderRegistry, SpringJointPropsRegistry, SpringNode, SpringNodeRegistry,
    };
    use crate::vrm::spring_bone::{SpringJointProps, SpringJointState, SpringRoot};
    use bevy::app::App;
    use bevy::core::Name;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::math::Vec3;
    use bevy::prelude::{BuildChildren, Commands, Entity, Transform};
    use bevy::utils::default;

    #[test]
    fn test_attach_spring_root() -> TestResult {
        let mut app = test_app();
        let head: Entity = app.world_mut().run_system_once(|mut commands: Commands| {
            let head = commands.spawn(Name::new("head")).id();
            commands
                .spawn((
                    SpringNodeRegistry(vec![SpringNode {
                        center: None,
                        joints: vec![Name::new("head")],
                        ..default()
                    }]),
                    HumanoidBoneRegistry::default(),
                ))
                .with_child(Name::new("Root"))
                .add_child(head);
            head
        })?;
        app.update();

        app.world_mut().run_system_once(attach_spring_roots)?;
        app.update();

        let query = app
            .world_mut()
            .query::<(Entity, &SpringRoot)>()
            .iter(app.world_mut())
            .next()
            .unwrap();
        assert_eq!(
            query,
            (
                head,
                &SpringRoot {
                    joints: vec![head,],
                    ..default()
                }
            )
        );
        success!()
    }

    #[test]
    fn set_center_node_spring_root() -> TestResult {
        let mut app = test_app();
        let (center, head): (Entity, Entity) =
            app.world_mut().run_system_once(|mut commands: Commands| {
                let center = commands.spawn(Name::new("center")).id();
                let head = commands.spawn(Name::new("head")).id();
                commands
                    .spawn((
                        SpringNodeRegistry(vec![SpringNode {
                            center: Some(Name::new("center")),
                            joints: vec![Name::new("head")],
                            ..default()
                        }]),
                        HumanoidBoneRegistry::default(),
                    ))
                    .with_child(Name::new("Root"))
                    .add_child(head)
                    .add_child(center);
                (center, head)
            })?;
        app.update();

        app.world_mut().run_system_once(attach_spring_roots)?;
        app.update();

        let query = app
            .world_mut()
            .query::<(Entity, &SpringRoot)>()
            .iter(app.world_mut())
            .next()
            .unwrap();
        assert_eq!(
            query,
            (
                head,
                &SpringRoot {
                    center_node: Some(center),
                    joints: vec![head,],
                    ..default()
                }
            )
        );
        success!()
    }

    #[test]
    fn test_init_spring_joint_state() -> TestResult {
        let mut app = test_app();
        let head: Entity = app.world_mut().run_system_once(|mut commands: Commands| {
            let head = commands
                .spawn((Name::new("head"), Transform::from_xyz(0.0, 0.0, 0.0)))
                .id();
            commands
                .spawn((
                    SpringNodeRegistry(vec![SpringNode {
                        center: None,
                        joints: vec![Name::new("head"), Name::new("tail")],
                        ..default()
                    }]),
                    HumanoidBoneRegistry::default(),
                ))
                .with_child(Name::new("Root"))
                .add_child(head)
                .with_child((Name::new("tail"), Transform::from_xyz(0.0, 2.0, 0.0)));
            head
        })?;
        app.update();

        app.world_mut().run_system_once(attach_spring_roots)?;
        app.update();

        app.world_mut().run_system_once(init_spring_joint_states)?;
        app.update();

        let states = app
            .world_mut()
            .query::<(Entity, &SpringJointState)>()
            .iter(app.world_mut())
            .collect::<Vec<_>>();

        assert_eq!(
            states,
            vec![(
                head,
                &SpringJointState {
                    current_tail: Vec3::new(0.0, 2.0, 0.0),
                    prev_tail: Vec3::new(0.0, 2.0, 0.0),
                    bone_axis: Vec3::new(0.0, 2.0, 0.0).normalize(),
                    bone_length: 2.0,
                    initial_local_matrix: Transform::from_xyz(0.0, 2.0, 0.0).compute_matrix(),
                    ..default()
                },
            )]
        );
        success!()
    }

    #[test]
    fn has_been_attached_joint_props() -> TestResult {
        let mut app = test_app();
        spawn_registry(&mut app)?;

        app.world_mut().run_system_once(attach_joint_props)?;
        assert!(app
            .world_mut()
            .query::<&AttachedJointProps>()
            .get_single(app.world())
            .is_ok());

        Ok(())
    }

    #[test]
    fn has_been_attached_collider_shapes() -> TestResult {
        let mut app = test_app();
        spawn_registry(&mut app)?;

        app.world_mut().run_system_once(attach_collider_shapes)?;
        assert!(app
            .world_mut()
            .query::<&AttachedColliderShapes>()
            .get_single(app.world())
            .is_ok());

        Ok(())
    }

    #[test]
    fn has_been_attached_spring_roots() -> TestResult {
        let mut app = test_app();
        spawn_registry(&mut app)?;

        app.world_mut().run_system_once(attach_spring_roots)?;
        assert!(app
            .world_mut()
            .query::<&AttachedSpringRoots>()
            .get_single(app.world())
            .is_ok());

        Ok(())
    }

    fn spawn_registry(app: &mut App) -> TestResult {
        app.world_mut().run_system_once(|mut commands: Commands| {
            commands
                .spawn((
                    SpringNodeRegistry(vec![SpringNode {
                        center: None,
                        joints: vec![Name::new("head")],
                        ..default()
                    }]),
                    SpringColliderRegistry(
                        [(Name::new("head"), ColliderShape::default())]
                            .into_iter()
                            .collect(),
                    ),
                    SpringJointPropsRegistry(
                        [(Name::new("head"), SpringJointProps::default())]
                            .into_iter()
                            .collect(),
                    ),
                    HumanoidBoneRegistry::default(),
                ))
                .with_child(Name::new("Root"))
                .with_child(Name::new("head"));
        })?;
        Ok(())
    }
}
