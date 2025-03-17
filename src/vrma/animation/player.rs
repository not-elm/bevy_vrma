use crate::system_param::vrm_animation_players::VrmAnimationPlayers;
use crate::vrma::retarget::CurrentRetargeting;
use crate::vrma::{RetargetSource, VrmaEntity};
use bevy::app::{App, Plugin};
use bevy::prelude::{Children, Commands, Entity, Event, Query, Reflect, Trigger};

#[derive(Event, Debug, Reflect)]
pub struct PlayAnimation {
    pub repeat: bool,
}

#[derive(Event, Debug, Reflect)]
pub struct StopAnimation;

pub struct VrmaAnimationPlayPlugin;

impl Plugin for VrmaAnimationPlayPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PlayAnimation>()
            .register_type::<StopAnimation>()
            .add_event::<PlayAnimation>()
            .add_event::<StopAnimation>()
            .add_observer(observe_play_animation)
            .add_observer(observe_stop_animation);
    }
}

fn observe_play_animation(
    trigger: Trigger<PlayAnimation>,
    mut commands: Commands,
    mut players: VrmAnimationPlayers,
    entities: Query<(Option<&Children>, Option<&RetargetSource>)>,
) {
    players.play(VrmaEntity(trigger.entity()), trigger.repeat);

    foreach_children(
        &mut commands,
        trigger.entity(),
        &entities,
        &|commands, entity, target_source| {
            if target_source.is_some() {
                commands.entity(entity).insert(CurrentRetargeting);
            }
        },
    );
}

fn observe_stop_animation(
    trigger: Trigger<StopAnimation>,
    mut commands: Commands,
    mut players: VrmAnimationPlayers,
    entities: Query<(Option<&Children>, Option<&RetargetSource>)>,
) {
    players.stop(VrmaEntity(trigger.entity()));
    foreach_children(
        &mut commands,
        trigger.entity(),
        &entities,
        &|commands, entity, retargeting_marker| {
            if retargeting_marker.is_some() {
                commands.entity(entity).remove::<CurrentRetargeting>();
            }
        },
    );
}

fn foreach_children(
    commands: &mut Commands,
    entity: Entity,
    entities: &Query<(Option<&Children>, Option<&RetargetSource>)>,
    f: &impl Fn(&mut Commands, Entity, Option<&RetargetSource>),
) {
    let Ok((children, bone_to)) = entities.get(entity) else {
        return;
    };
    f(commands, entity, bone_to);
    if let Some(children) = children {
        for child in children.iter() {
            foreach_children(commands, *child, entities, f);
        }
    }
}
