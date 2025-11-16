use bevy::prelude::SystemSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Clone, Copy)]
pub enum VrmSystemSets {
    /// Node constraints processing.
    Constraints,

    /// Manual transform propagation after Constraints.
    /// This propagates Transform changes from Constraints to `GlobalTransform`.
    PropagateAfterConstraints,

    /// Look-at binding processing.
    GazeControl,

    /// Expression binding processing.
    Expressions,

    /// Manual transform propagation after Expressions.
    /// This propagates Transform changes from `GazeControl` and Expressions to `GlobalTransform`.
    PropagateAfterExpressions,

    /// This is used for spring bones.
    SpringBone,

    /// This is used to determine whether to send a [`RequestRedraw`](bevy::window::RequestRedraw).
    DetermineRedraw,
}
