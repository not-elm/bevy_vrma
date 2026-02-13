//!  This module handles the retargeting of expressions from a VRM model to a mascot model.

use crate::system_set::VrmSystemSets;
use crate::vrm::VrmExpression;
use crate::vrm::expressions::{BindExpressionNode, ExpressionOverride, RetargetExpressionNodes};
use crate::vrma::gltf::extensions::VrmaExtensions;
use bevy::app::App;
use bevy::prelude::*;

pub(in crate::vrma) struct VrmaRetargetExpressionsPlugin;

impl Plugin for VrmaRetargetExpressionsPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<RetargetExpressionNodes>()
            .register_type::<BindExpressionNode>()
            .add_systems(
                PostUpdate,
                bind_expressions
                    .in_set(VrmSystemSets::Expressions)
                    .after(VrmSystemSets::GazeControl),
            );
    }
}

#[derive(Component, Deref, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub(crate) struct VrmaExpressionNames(Vec<VrmExpression>);

impl VrmaExpressionNames {
    pub fn new(extensions: &VrmaExtensions) -> Self {
        let Some(expressions) = extensions.vrmc_vrm_animation.expressions.as_ref() else {
            return Self(Vec::default());
        };
        Self(
            expressions
                .preset
                .keys()
                .map(|expression| VrmExpression(expression.clone()))
                .collect(),
        )
    }
}

fn bind_expressions(
    mut expressions: Query<&mut MorphWeights>,
    rig_expressions: Query<
        (&Transform, &RetargetExpressionNodes, Option<&ExpressionOverride>),
        Or<(Changed<Transform>, Changed<ExpressionOverride>)>,
    >,
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

#[cfg(test)]
mod tests {
    use crate::tests::{TestResult, test_app};
    use crate::vrm::expressions::ExpressionOverride;
    use bevy::prelude::*;

    use super::*;

    #[test]
    fn test_bind_expressions_prefers_override() -> TestResult {
        let mut app = test_app();
        app.add_plugins(VrmaRetargetExpressionsPlugin);

        // Create a mesh entity with morph weights
        let mesh_entity = app
            .world_mut()
            .spawn(MorphWeights::new(vec![0.0], None)?)
            .id();

        // Create an expression entity with VRMA value (Transform) and override
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.3, 0.0, 0.0)),
            RetargetExpressionNodes(vec![BindExpressionNode {
                expression_entity: mesh_entity,
                index: 0,
            }]),
            ExpressionOverride(0.9),
        ));
        app.update();

        let morph = app.world().get::<MorphWeights>(mesh_entity).unwrap();
        assert!(
            (morph.weights()[0] - 0.9).abs() < f32::EPSILON,
            "Expected override value 0.9, got {}",
            morph.weights()[0]
        );
        Ok(())
    }

    #[test]
    fn test_bind_expressions_falls_back_to_transform() -> TestResult {
        let mut app = test_app();
        app.add_plugins(VrmaRetargetExpressionsPlugin);

        let mesh_entity = app
            .world_mut()
            .spawn(MorphWeights::new(vec![0.0], None)?)
            .id();

        // No ExpressionOverride — should use Transform.translation.x
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.5, 0.0, 0.0)),
            RetargetExpressionNodes(vec![BindExpressionNode {
                expression_entity: mesh_entity,
                index: 0,
            }]),
        ));
        app.update();

        let morph = app.world().get::<MorphWeights>(mesh_entity).unwrap();
        assert!(
            (morph.weights()[0] - 0.5).abs() < f32::EPSILON,
            "Expected VRMA value 0.5, got {}",
            morph.weights()[0]
        );
        Ok(())
    }
}
