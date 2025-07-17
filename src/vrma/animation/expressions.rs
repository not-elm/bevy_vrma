//!  This module handles the retargeting of expressions from a VRM model to a mascot model.

use crate::system_set::VrmSystemSets;
use crate::vrm::VrmExpression;
use crate::vrm::expressions::{BindExpressionNode, RetargetExpressionNodes};
use crate::vrma::gltf::extensions::VrmaExtensions;
use bevy::app::App;
use bevy::prelude::TransformSystem::TransformPropagate;
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
                    .in_set(VrmSystemSets::Retarget)
                    .before(TransformPropagate),
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
    rig_expressions: Query<(&Transform, &RetargetExpressionNodes), Changed<Transform>>,
) {
    for (tf, RetargetExpressionNodes(binds)) in rig_expressions.iter() {
        // VRMA uses x coordinate to represent expression weight.
        let weight = tf.translation.x;
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
