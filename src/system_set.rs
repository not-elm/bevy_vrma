use bevy::prelude::SystemSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Clone, Copy)]
pub enum VrmSystemSets {
    /// Node constraints processing.
    Constraints,

    /// Look-at binding processing.
    GazeControl,

    /// Expression binding processing.
    Expressions,

    /// This is used for spring bones.
    SpringBone,

    /// This is used to determine whether to send a [`RequestRedraw`](bevy::window::RequestRedraw).
    DetermineRedraw,
}
