pub(crate) mod animation_graph;
mod bone_rotation;
mod bone_translation;
pub(crate) mod expressions;
mod play;

use crate::prelude::VrmSystemSets;
use crate::vrma::RetargetSource;
use crate::vrma::animation::animation_graph::VrmaAnimationGraphPlugin;
use crate::vrma::animation::expressions::VrmaRetargetExpressionsPlugin;
use crate::vrma::animation::play::VrmaAnimationPlayPlugin;
use bevy::app::App;
use bevy::prelude::*;
use bevy::window::RequestRedraw;

pub mod prelude {
    pub use crate::vrma::animation::{
        VrmaAnimationPlayers,
        play::{PlayVrma, StopVrma},
    };
}

pub(super) struct VrmaAnimationPlayersPlugin;

impl Plugin for VrmaAnimationPlayersPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<VrmaAnimationPlayers>()
            .add_plugins((
                VrmaAnimationGraphPlugin,
                VrmaAnimationPlayPlugin,
                VrmaRetargetExpressionsPlugin,
            ))
            .add_systems(
                PostUpdate,
                request_redraw
                    .in_set(VrmSystemSets::DetermineRedraw)
                    .run_if(any_playing_animations.or(any_update_spring_joints)),
            );
    }
}

/// After spawn the vrma, the animation player will be spawned.
/// This component is used to hold that entity in the root entity.
#[derive(Component, Debug, Deref, DerefMut, Default, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct VrmaAnimationPlayers(pub Vec<Entity>);

fn any_playing_animations(players: Query<&AnimationPlayer, With<RetargetSource>>) -> bool {
    players.iter().any(|p| !p.all_finished())
}

fn any_update_spring_joints(spring_joints: Query<&Transform, Changed<Transform>>) -> bool {
    spring_joints
        .iter()
        .any(|tf| 0.1 < tf.rotation.angle_between(Quat::IDENTITY))
}

fn request_redraw(mut request: EventWriter<RequestRedraw>) {
    request.write(RequestRedraw);
}
