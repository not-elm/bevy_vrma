use crate::prelude::ChildSearcher;
use crate::system_set::VrmSystemSets;
use crate::vrm::gltf::extensions::VrmExtensions;
use crate::vrm::gltf::extensions::vrmc_vrm::MorphTargetBind;
use crate::vrm::{Vrm, VrmExpression};
use crate::vrma::RetargetSource;
use bevy::animation::{AnimatedBy, AnimationTargetId};
use bevy::app::Plugin;
use bevy::asset::{Assets, Handle};
use bevy::gltf::GltfNode;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

#[derive(Reflect, Debug, Clone)]
pub(crate) struct ExpressionNode {
    pub name: Name,
    pub morph_target_index: usize,
}

/// Cached mapping from expression name to expression entity.
/// Built during VRM initialization. Use this to query available expressions.
#[derive(Component, Deref, Reflect)]
pub struct ExpressionEntityMap(pub HashMap<VrmExpression, Entity>);

/// Override weight for a single expression entity.
/// Inserted by [`SetExpressions`] or [`ModifyExpressions`], removed by [`ClearExpressions`].
#[derive(Component, Reflect)]
#[reflect(Component)]
pub(crate) struct ExpressionOverride(pub f32);

/// Sets expression weights on a VRM model, **replacing all previous overrides**.
///
/// Trigger this event to directly control facial expressions.
/// Expression weights are clamped to `0.0..=1.0`.
/// Expressions not included in this call will return to VRMA animation control.
///
/// For partial updates that preserve existing overrides, see [`ModifyExpressions`].
///
/// **Note**: Triggering both `SetExpressions` and [`ModifyExpressions`]
/// on the same entity in the same frame produces undefined results.
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_vrm1::prelude::*;
///
/// fn set_happy(mut commands: Commands, vrms: Query<Entity, With<Vrm>>) {
///     for vrm in vrms.iter() {
///         commands.trigger(SetExpressions::single(vrm, "happy", 1.0));
///     }
/// }
/// ```
#[derive(EntityEvent, Debug)]
pub struct SetExpressions {
    #[event_target]
    pub entity: Entity,
    pub weights: HashMap<VrmExpression, f32>,
}

impl SetExpressions {
    /// Creates a [`SetExpressions`] event for a single expression.
    pub fn single(
        entity: Entity,
        expression: impl Into<VrmExpression>,
        weight: f32,
    ) -> Self {
        Self {
            entity,
            weights: [(expression.into(), weight)].into_iter().collect(),
        }
    }

    /// Creates a [`SetExpressions`] event from an iterator of expression-weight pairs.
    pub fn from_iter(
        entity: Entity,
        iter: impl IntoIterator<Item = (impl Into<VrmExpression>, f32)>,
    ) -> Self {
        Self {
            entity,
            weights: iter.into_iter().map(|(e, w)| (e.into(), w)).collect(),
        }
    }
}

/// Modifies specific expression weights without affecting others (partial update).
///
/// Unlike [`SetExpressions`] which replaces all overrides,
/// this only inserts/updates the specified expressions.
/// Existing overrides not mentioned in this call remain unchanged.
///
/// **Note**: Triggering both [`SetExpressions`] and `ModifyExpressions`
/// on the same entity in the same frame produces undefined results
/// due to Bevy observer ordering not being guaranteed.
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_vrm1::prelude::*;
///
/// fn add_blink(mut commands: Commands, vrms: Query<Entity, With<Vrm>>) {
///     for vrm in vrms.iter() {
///         // Only modifies "blink", leaves other overrides (e.g. "happy") intact
///         commands.trigger(ModifyExpressions::single(vrm, "blink", 1.0));
///     }
/// }
/// ```
#[derive(EntityEvent, Debug)]
pub struct ModifyExpressions {
    #[event_target]
    pub entity: Entity,
    pub weights: HashMap<VrmExpression, f32>,
}

impl ModifyExpressions {
    /// Creates a [`ModifyExpressions`] event for a single expression.
    pub fn single(
        entity: Entity,
        expression: impl Into<VrmExpression>,
        weight: f32,
    ) -> Self {
        Self {
            entity,
            weights: [(expression.into(), weight)].into_iter().collect(),
        }
    }

    /// Creates a [`ModifyExpressions`] event from an iterator of expression-weight pairs.
    pub fn from_iter(
        entity: Entity,
        iter: impl IntoIterator<Item = (impl Into<VrmExpression>, f32)>,
    ) -> Self {
        Self {
            entity,
            weights: iter.into_iter().map(|(e, w)| (e.into(), w)).collect(),
        }
    }
}

/// Clears all expression overrides, returning control to VRMA animation.
///
/// After triggering this event, expressions previously set by [`SetExpressions`]
/// or [`ModifyExpressions`] will be controlled by VRMA animation again.
#[derive(EntityEvent, Debug)]
pub struct ClearExpressions {
    #[event_target]
    pub entity: Entity,
}

#[derive(EntityEvent)]
pub(crate) struct RequestInitializeExpressions(pub(crate) Entity);

#[derive(Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub(crate) struct BindExpressionNode {
    pub expression_entity: Entity,
    pub index: usize,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub(crate) struct RetargetExpressionNodes(pub(crate) Vec<BindExpressionNode>);

#[derive(Component, Deref, Reflect)]
pub(crate) struct VrmExpressionRegistry(pub(crate) HashMap<VrmExpression, Vec<ExpressionNode>>);

impl VrmExpressionRegistry {
    pub fn new(
        extensions: &VrmExtensions,
        node_assets: &Assets<GltfNode>,
        nodes: &[Handle<GltfNode>],
    ) -> Self {
        let Some(expressions) = extensions.vrmc_vrm.expressions.as_ref() else {
            return Self(HashMap::default());
        };
        Self(
            expressions
                .preset
                .iter()
                .filter_map(|(preset_name, preset)| {
                    let binds = preset.morph_target_binds.as_ref()?;
                    let node = binds
                        .iter()
                        .filter_map(|bind| convert_to_node(bind, node_assets, nodes))
                        .collect::<Vec<_>>();
                    Some((VrmExpression(preset_name.clone()), node))
                })
                .collect(),
        )
    }
}

pub(crate) struct VrmExpressionPlugin;

impl Plugin for VrmExpressionPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<BindExpressionNode>()
            .register_type::<RetargetExpressionNodes>()
            .register_type::<VrmExpressionRegistry>()
            .register_type::<ExpressionEntityMap>()
            .register_type::<ExpressionOverride>()
            .add_observer(apply_initialize_expressions)
            .add_observer(apply_set_expressions)
            .add_observer(apply_clear_expressions)
            .add_observer(apply_modify_expressions)
            .add_systems(
                PostUpdate,
                bind_expressions
                    .in_set(VrmSystemSets::Expressions)
                    .after(VrmSystemSets::GazeControl),
            );
    }
}

fn convert_to_node(
    bind: &MorphTargetBind,
    node_assets: &Assets<GltfNode>,
    nodes: &[Handle<GltfNode>],
) -> Option<ExpressionNode> {
    let node_handle = nodes.get(bind.node)?;
    let node = node_assets.get(node_handle)?;
    Some(ExpressionNode {
        name: Name::new(node.name.clone()),
        morph_target_index: bind.index,
    })
}

fn apply_initialize_expressions(
    trigger: On<RequestInitializeExpressions>,
    mut commands: Commands,
    expressions: Query<&VrmExpressionRegistry>,
    searcher: ChildSearcher,
) {
    let vrm_entity = trigger.event_target();
    let expressions_root = commands.spawn(Name::new(Vrm::EXPRESSIONS_ROOT)).id();
    commands.entity(vrm_entity).add_child(expressions_root);

    let Ok(registry) = expressions.get(vrm_entity) else {
        commands
            .entity(vrm_entity)
            .insert(ExpressionEntityMap(HashMap::default()));
        return;
    };
    let mut entity_map = HashMap::default();
    for (expression, nodes) in registry.iter() {
        let expression_entity = commands
            .spawn((
                Name::new(expression.to_string()),
                RetargetSource,
                Transform::default(),
                AnimationPlayer::default(),
                RetargetExpressionNodes(obtain_expression_nodes(vrm_entity, &searcher, nodes)),
            ))
            .id();
        commands.entity(expression_entity).insert((
            AnimationTargetId::from_name(&Name::new(expression.to_string())),
            AnimatedBy(expression_entity),
        ));
        commands
            .entity(expressions_root)
            .add_child(expression_entity);
        entity_map.insert(expression.clone(), expression_entity);
    }
    commands
        .entity(vrm_entity)
        .insert(ExpressionEntityMap(entity_map));
}

fn bind_expressions(
    mut expressions: Query<&mut MorphWeights>,
    rig_expressions: Query<(
        &Transform,
        &RetargetExpressionNodes,
        Option<&ExpressionOverride>,
    )>,
) {
    for (tf, RetargetExpressionNodes(binds), maybe_override) in rig_expressions.iter() {
        let weight = match maybe_override {
            Some(ExpressionOverride(w)) => *w,
            None => tf.translation.x,
        };
        for BindExpressionNode {
            expression_entity,
            index,
        } in binds.iter()
        {
            if let Ok(mut morph_weights) = expressions.get_mut(*expression_entity) {
                morph_weights.weights_mut()[*index] = weight;
            }
        }
    }
}

fn apply_set_expressions(
    trigger: On<SetExpressions>,
    cache: Query<&ExpressionEntityMap>,
    mut commands: Commands,
) {
    let vrm_entity = trigger.event_target();
    let Ok(map) = cache.get(vrm_entity) else {
        #[cfg(feature = "log")]
        warn!(
            "SetExpressions: ExpressionEntityMap not found for entity {:?}. VRM may not be initialized yet.",
            vrm_entity
        );
        return;
    };
    // Remove overrides not present in the new weights so that
    // each SetExpressions call fully replaces the previous state.
    for (&expr_entity, expression) in map.0.iter().map(|(e, id)| (id, e)) {
        if !trigger.weights.contains_key(expression) {
            commands.entity(expr_entity).remove::<ExpressionOverride>();
        }
    }
    for (expression, weight) in trigger.weights.iter() {
        let Some(&expr_entity) = map.0.get(expression) else {
            #[cfg(feature = "log")]
            warn!("SetExpressions: expression '{}' not found", expression);
            continue;
        };
        commands
            .entity(expr_entity)
            .insert(ExpressionOverride(weight.clamp(0.0, 1.0)));
    }
}

fn apply_clear_expressions(
    trigger: On<ClearExpressions>,
    cache: Query<&ExpressionEntityMap>,
    mut commands: Commands,
) {
    let vrm_entity = trigger.event_target();
    let Ok(map) = cache.get(vrm_entity) else {
        return;
    };
    for &expr_entity in map.0.values() {
        commands.entity(expr_entity).remove::<ExpressionOverride>();
    }
}

fn apply_modify_expressions(
    trigger: On<ModifyExpressions>,
    cache: Query<&ExpressionEntityMap>,
    mut commands: Commands,
) {
    let vrm_entity = trigger.event_target();
    let Ok(map) = cache.get(vrm_entity) else {
        #[cfg(feature = "log")]
        warn!(
            "ModifyExpressions: ExpressionEntityMap not found for entity {:?}. VRM may not be initialized yet.",
            vrm_entity
        );
        return;
    };
    for (expression, weight) in trigger.weights.iter() {
        let Some(&expr_entity) = map.0.get(expression) else {
            #[cfg(feature = "log")]
            warn!("ModifyExpressions: expression '{}' not found", expression);
            continue;
        };
        commands
            .entity(expr_entity)
            .insert(ExpressionOverride(weight.clamp(0.0, 1.0)));
    }
}

fn obtain_expression_nodes(
    vrm_entity: Entity,
    searcher: &ChildSearcher,
    nodes: &[ExpressionNode],
) -> Vec<BindExpressionNode> {
    nodes
        .iter()
        .flat_map(|node| {
            Some(BindExpressionNode {
                expression_entity: searcher.find_from_name(vrm_entity, &node.name)?,
                index: node.morph_target_index,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::tests::{TestResult, test_app};
    use crate::vrm::expressions::{
        ClearExpressions, ExpressionEntityMap, ExpressionNode, ExpressionOverride,
        ModifyExpressions, RequestInitializeExpressions, SetExpressions, VrmExpressionPlugin,
        VrmExpressionRegistry,
    };
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;

    #[test]
    fn test_obtain_expression_nodes() -> TestResult {
        let mut app = test_app();
        app.add_plugins(VrmExpressionPlugin);

        let vrm_entity = app
            .world_mut()
            .spawn((VrmExpressionRegistry(
                [(
                    VrmExpression::from("happy"),
                    vec![ExpressionNode {
                        name: Name::new("Test"),
                        morph_target_index: 0,
                    }],
                )]
                .into_iter()
                .collect(),
            ),))
            .with_children(|c| {
                c.spawn(Name::new("Test"));
            })
            .id();

        app.world_mut()
            .commands()
            .entity(vrm_entity)
            .trigger(RequestInitializeExpressions);
        app.update();

        app.world_mut()
            .run_system_once(move |s: ChildSearcher| s.find_expressions_root(vrm_entity))
            .expect("Failed to run system")
            .expect("Expression root not found");

        app.world_mut()
            .run_system_once(move |s: ChildSearcher| s.find_from_name(vrm_entity, "happy"))
            .expect("Failed to run system")
            .expect("Expression node not found");
        Ok(())
    }

    #[test]
    fn test_set_expressions() -> TestResult {
        let mut app = test_app();
        app.add_plugins(VrmExpressionPlugin);

        let vrm_entity = app
            .world_mut()
            .spawn((VrmExpressionRegistry(
                [(
                    VrmExpression::from("happy"),
                    vec![ExpressionNode {
                        name: Name::new("Test"),
                        morph_target_index: 0,
                    }],
                )]
                .into_iter()
                .collect(),
            ),))
            .with_children(|c| {
                c.spawn(Name::new("Test"));
            })
            .id();

        // Initialize expressions
        app.world_mut()
            .commands()
            .entity(vrm_entity)
            .trigger(RequestInitializeExpressions);
        app.update();

        // Set expression
        app.world_mut()
            .commands()
            .trigger(SetExpressions::single(vrm_entity, "happy", 0.8));
        app.update();

        // Find the expression entity via the map
        let map = app.world().get::<ExpressionEntityMap>(vrm_entity).unwrap();
        let expr_entity = *map.0.get(&VrmExpression::from("happy")).unwrap();

        let override_val = app
            .world()
            .get::<ExpressionOverride>(expr_entity)
            .expect("ExpressionOverride not found");
        assert!((override_val.0 - 0.8).abs() < f32::EPSILON);
        Ok(())
    }

    #[test]
    fn test_expression_entity_map_built_on_init() -> TestResult {
        let mut app = test_app();
        app.add_plugins(VrmExpressionPlugin);

        let vrm_entity = app
            .world_mut()
            .spawn((VrmExpressionRegistry(
                [(
                    VrmExpression::from("happy"),
                    vec![ExpressionNode {
                        name: Name::new("Test"),
                        morph_target_index: 0,
                    }],
                )]
                .into_iter()
                .collect(),
            ),))
            .with_children(|c| {
                c.spawn(Name::new("Test"));
            })
            .id();

        app.world_mut()
            .commands()
            .entity(vrm_entity)
            .trigger(RequestInitializeExpressions);
        app.update();

        let map = app
            .world()
            .get::<ExpressionEntityMap>(vrm_entity)
            .expect("ExpressionEntityMap not found");

        assert!(map.0.contains_key(&VrmExpression::from("happy")));
        Ok(())
    }

    #[test]
    fn test_clear_expressions() -> TestResult {
        let mut app = test_app();
        app.add_plugins(VrmExpressionPlugin);

        let vrm_entity = app
            .world_mut()
            .spawn((VrmExpressionRegistry(
                [(
                    VrmExpression::from("happy"),
                    vec![ExpressionNode {
                        name: Name::new("Test"),
                        morph_target_index: 0,
                    }],
                )]
                .into_iter()
                .collect(),
            ),))
            .with_children(|c| {
                c.spawn(Name::new("Test"));
            })
            .id();

        // Initialize
        app.world_mut()
            .commands()
            .entity(vrm_entity)
            .trigger(RequestInitializeExpressions);
        app.update();

        // Set expression
        app.world_mut()
            .commands()
            .trigger(SetExpressions::single(vrm_entity, "happy", 0.8));
        app.update();

        // Verify override exists
        let map = app.world().get::<ExpressionEntityMap>(vrm_entity).unwrap();
        let expr_entity = *map.0.get(&VrmExpression::from("happy")).unwrap();
        assert!(app.world().get::<ExpressionOverride>(expr_entity).is_some());

        // Clear expressions
        app.world_mut()
            .commands()
            .trigger(ClearExpressions { entity: vrm_entity });
        app.update();

        // Verify override removed
        assert!(app.world().get::<ExpressionOverride>(expr_entity).is_none());
        Ok(())
    }

    #[test]
    fn test_set_expressions_replaces_previous() -> TestResult {
        let mut app = test_app();
        app.add_plugins(VrmExpressionPlugin);

        let vrm_entity = app
            .world_mut()
            .spawn((VrmExpressionRegistry(
                [
                    (
                        VrmExpression::from("happy"),
                        vec![ExpressionNode {
                            name: Name::new("MeshA"),
                            morph_target_index: 0,
                        }],
                    ),
                    (
                        VrmExpression::from("angry"),
                        vec![ExpressionNode {
                            name: Name::new("MeshB"),
                            morph_target_index: 0,
                        }],
                    ),
                ]
                .into_iter()
                .collect(),
            ),))
            .with_children(|c| {
                c.spawn(Name::new("MeshA"));
                c.spawn(Name::new("MeshB"));
            })
            .id();

        app.world_mut()
            .commands()
            .entity(vrm_entity)
            .trigger(RequestInitializeExpressions);
        app.update();

        let map = app.world().get::<ExpressionEntityMap>(vrm_entity).unwrap();
        let happy_entity = *map.0.get(&VrmExpression::from("happy")).unwrap();
        let angry_entity = *map.0.get(&VrmExpression::from("angry")).unwrap();

        // Set happy
        app.world_mut()
            .commands()
            .trigger(SetExpressions::single(vrm_entity, "happy", 1.0));
        app.update();

        assert!(
            app.world()
                .get::<ExpressionOverride>(happy_entity)
                .is_some()
        );
        assert!(
            app.world()
                .get::<ExpressionOverride>(angry_entity)
                .is_none()
        );

        // Set angry — happy override should be removed
        app.world_mut()
            .commands()
            .trigger(SetExpressions::single(vrm_entity, "angry", 0.7));
        app.update();

        assert!(
            app.world()
                .get::<ExpressionOverride>(happy_entity)
                .is_none(),
            "Previous expression override should be removed"
        );
        let angry_override = app
            .world()
            .get::<ExpressionOverride>(angry_entity)
            .expect("New expression override not found");
        assert!((angry_override.0 - 0.7).abs() < f32::EPSILON);
        Ok(())
    }

    #[test]
    fn test_modify_expressions_preserves_existing() -> TestResult {
        let mut app = test_app();
        app.add_plugins(VrmExpressionPlugin);

        let vrm_entity = app
            .world_mut()
            .spawn((VrmExpressionRegistry(
                [
                    (
                        VrmExpression::from("happy"),
                        vec![ExpressionNode {
                            name: Name::new("MeshA"),
                            morph_target_index: 0,
                        }],
                    ),
                    (
                        VrmExpression::from("angry"),
                        vec![ExpressionNode {
                            name: Name::new("MeshB"),
                            morph_target_index: 0,
                        }],
                    ),
                ]
                .into_iter()
                .collect(),
            ),))
            .with_children(|c| {
                c.spawn(Name::new("MeshA"));
                c.spawn(Name::new("MeshB"));
            })
            .id();

        app.world_mut()
            .commands()
            .entity(vrm_entity)
            .trigger(RequestInitializeExpressions);
        app.update();

        let map = app.world().get::<ExpressionEntityMap>(vrm_entity).unwrap();
        let happy_entity = *map.0.get(&VrmExpression::from("happy")).unwrap();
        let angry_entity = *map.0.get(&VrmExpression::from("angry")).unwrap();

        // Set happy via SetExpressions
        app.world_mut()
            .commands()
            .trigger(SetExpressions::single(vrm_entity, "happy", 1.0));
        app.update();

        assert!(
            app.world()
                .get::<ExpressionOverride>(happy_entity)
                .is_some()
        );

        // Modify angry — happy override should be preserved
        app.world_mut()
            .commands()
            .trigger(ModifyExpressions::single(vrm_entity, "angry", 0.7));
        app.update();

        // happy override is still present
        let happy_override = app
            .world()
            .get::<ExpressionOverride>(happy_entity)
            .expect("Existing override should be preserved by ModifyExpressions");
        assert!((happy_override.0 - 1.0).abs() < f32::EPSILON);

        // angry override was added
        let angry_override = app
            .world()
            .get::<ExpressionOverride>(angry_entity)
            .expect("ModifyExpressions should add new override");
        assert!((angry_override.0 - 0.7).abs() < f32::EPSILON);
        Ok(())
    }
}
