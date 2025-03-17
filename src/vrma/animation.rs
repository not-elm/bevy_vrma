pub mod player;
mod setup;

use crate::vrma::animation::player::VrmaAnimationPlayPlugin;
use crate::vrma::animation::setup::VrmaAnimationSetupPlugin;
use bevy::app::App;
use bevy::asset::Handle;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct VrmaAnimationPlayersPlugin;

impl Plugin for VrmaAnimationPlayersPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AnimationPlayerEntities>()
            .add_plugins((VrmaAnimationSetupPlugin, VrmaAnimationPlayPlugin));
    }
}

/// After spawn the vrma, bone and each expression animation players will be spawned.
/// This component is used to hold those entities in the root entity.
#[derive(Component, Debug, Reflect, Deref, DerefMut, Default, Serialize, Deserialize)]
#[reflect(Component, Debug, Serialize, Deserialize)]
pub struct AnimationPlayerEntities(pub Vec<Entity>);

#[derive(Component, Default)]
pub struct VrmAnimationGraph {
    pub handle: Handle<AnimationGraph>,
    pub nodes: Vec<AnimationNodeIndex>,
}

impl VrmAnimationGraph {
    pub fn new(
        clip: impl IntoIterator<Item = Handle<AnimationClip>>,
        animation_graphs: &mut Assets<AnimationGraph>,
    ) -> Self {
        let (graph, nodes) = AnimationGraph::from_clips(clip);
        let handle = animation_graphs.add(graph);

        Self { handle, nodes }
    }
}
