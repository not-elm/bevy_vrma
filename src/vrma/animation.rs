pub(crate) mod animation_graph;
mod bone_rotation;
mod bone_translation;
pub(crate) mod expressions;
mod play;

use crate::vrma::animation::animation_graph::VrmaAnimationGraphPlugin;
use crate::vrma::animation::expressions::VrmaRetargetExpressionsPlugin;
use crate::vrma::animation::play::VrmaAnimationPlayPlugin;
use crate::vrma::RetargetSource;
use bevy::app::App;
use bevy::prelude::*;
use bevy::window::RequestRedraw;

pub mod prelude {
    pub use crate::vrma::animation::{
        play::{PlayVrma, StopVrma},
        VrmaAnimationPlayers,
    };
}

pub struct VrmaAnimationPlayersPlugin;

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
            .add_systems(Update, request_redraw.run_if(playing_animation));
    }
}

/// After spawn the vrma, the animation player will be spawned.
/// This component is used to hold that entity in the root entity.
#[derive(Component, Debug, Deref, DerefMut, Default, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct VrmaAnimationPlayers(pub Vec<Entity>);

fn playing_animation(
    changed_bones: Query<Entity, (Changed<Transform>, With<RetargetSource>)>
) -> bool {
    !changed_bones.is_empty()
}

fn request_redraw(mut request: EventWriter<RequestRedraw>) {
    request.write(RequestRedraw);
}
