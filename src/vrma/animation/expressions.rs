//!  This module handles the retargeting of expressions from a VRM model to a mascot model.

use crate::vrm::VrmExpression;
use crate::vrma::gltf::extensions::VrmaExtensions;
use bevy::app::App;
use bevy::prelude::*;

pub(in crate::vrma) struct VrmaRetargetExpressionsPlugin;

impl Plugin for VrmaRetargetExpressionsPlugin {
    fn build(
        &self,
        _app: &mut App,
    ) {
        // bind_expressions system is now registered in VrmExpressionPlugin
        // so it works with or without VrmaPlugin.
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

#[cfg(test)]
mod tests {
    use crate::tests::{TestResult, test_app};
    use crate::vrm::expressions::{
        BindExpressionNode, ExpressionCategory, ExpressionCategoryTag, ExpressionOverride,
        ExpressionOverrideSettings, ExpressionOverrideType, RetargetExpressionNodes,
        VrmExpressionPlugin,
    };
    use bevy::prelude::*;

    fn default_override_settings() -> ExpressionOverrideSettings {
        ExpressionOverrideSettings {
            override_mouth: ExpressionOverrideType::None,
            override_blink: ExpressionOverrideType::None,
            override_look_at: ExpressionOverrideType::None,
        }
    }

    #[test]
    fn test_bind_expressions_prefers_override() -> TestResult {
        let mut app = test_app();
        app.add_plugins(VrmExpressionPlugin);

        let mesh_entity = app
            .world_mut()
            .spawn(MorphWeights::new(vec![0.0], None)?)
            .id();

        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.3, 0.0, 0.0)),
            RetargetExpressionNodes(vec![BindExpressionNode {
                expression_entity: mesh_entity,
                index: 0,
                weight: 1.0,
            }]),
            ExpressionCategoryTag(ExpressionCategory::Other),
            default_override_settings(),
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
        app.add_plugins(VrmExpressionPlugin);

        let mesh_entity = app
            .world_mut()
            .spawn(MorphWeights::new(vec![0.0], None)?)
            .id();

        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.5, 0.0, 0.0)),
            RetargetExpressionNodes(vec![BindExpressionNode {
                expression_entity: mesh_entity,
                index: 0,
                weight: 1.0,
            }]),
            ExpressionCategoryTag(ExpressionCategory::Other),
            default_override_settings(),
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
