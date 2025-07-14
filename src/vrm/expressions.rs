use crate::prelude::ChildSearcher;
use crate::vrm::gltf::extensions::VrmExtensions;
use crate::vrm::gltf::extensions::vrmc_vrm::MorphTargetBind;
use crate::vrm::{Vrm, VrmExpression};
use crate::vrma::RetargetSource;
use bevy::animation::{AnimationTarget, AnimationTargetId};
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

#[derive(Event)]
pub(crate) struct RequestInitializeExpressions;

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
            .add_observer(apply_initialize_expressions);
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
    trigger: Trigger<RequestInitializeExpressions>,
    mut commands: Commands,
    expressions: Query<&VrmExpressionRegistry>,
    searcher: ChildSearcher,
) {
    let vrm_entity = trigger.target();
    let expressions_root = commands.spawn(Name::new(Vrm::EXPRESSIONS_ROOT)).id();
    commands.entity(vrm_entity).add_child(expressions_root);

    let Ok(registry) = expressions.get(vrm_entity) else {
        return;
    };
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
        commands.entity(expression_entity).insert(AnimationTarget {
            id: AnimationTargetId::from_name(&Name::new(expression.to_string())),
            player: expression_entity,
        });
        commands
            .entity(expressions_root)
            .add_child(expression_entity);
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
        ExpressionNode, RequestInitializeExpressions, VrmExpressionPlugin, VrmExpressionRegistry,
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
            .run_system_once(move |s: ChildSearcher| s.find_expressions_root(vrm_entity))?
            .expect("Expression root not found");

        app.world_mut()
            .run_system_once(move |s: ChildSearcher| s.find_from_name(vrm_entity, "happy"))?
            .expect("Expression node not found");
        Ok(())
    }
}
