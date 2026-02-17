use bevy::prelude::*;

/// Optional body tracking that makes head, neck, chest, and spine bones
/// follow the LookAt target. Insert alongside [`LookAt`] to enable.
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_vrm1::prelude::*;
///
/// fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
///     commands.spawn((
///         VrmHandle(asset_server.load("model.vrm")),
///         LookAt::Cursor,
///         BodyTracking::default(),
///     ));
/// }
/// ```
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct BodyTracking {
    /// Fraction of total gaze angle applied to head bone (0.0-1.0).
    pub head_weight: f32,
    /// Fraction of total gaze angle applied to neck bone (0.0-1.0).
    pub neck_weight: f32,
    /// Fraction of total gaze angle applied to chest bone (0.0-1.0).
    pub chest_weight: f32,
    /// Fraction of total gaze angle applied to spine bone (0.0-1.0).
    pub spine_weight: f32,

    /// Maximum head yaw in degrees.
    pub head_yaw_max: f32,
    /// Maximum head pitch in degrees.
    pub head_pitch_max: f32,
    /// Maximum neck yaw in degrees.
    pub neck_yaw_max: f32,
    /// Maximum neck pitch in degrees.
    pub neck_pitch_max: f32,
    /// Maximum chest yaw in degrees.
    pub chest_yaw_max: f32,
    /// Maximum chest pitch in degrees. Set to 0.0 for yaw-only.
    pub chest_pitch_max: f32,
    /// Maximum spine yaw in degrees.
    pub spine_yaw_max: f32,
    /// Maximum spine pitch in degrees. Set to 0.0 for yaw-only.
    pub spine_pitch_max: f32,

    /// Smoothing speed. Higher values = faster response. 0.0 = instant (no smoothing).
    pub smoothing: f32,
}

impl Default for BodyTracking {
    fn default() -> Self {
        Self {
            head_weight: 0.4,
            neck_weight: 0.25,
            chest_weight: 0.2,
            spine_weight: 0.15,
            head_yaw_max: 40.0,
            head_pitch_max: 30.0,
            neck_yaw_max: 25.0,
            neck_pitch_max: 20.0,
            chest_yaw_max: 20.0,
            chest_pitch_max: 0.0,
            spine_yaw_max: 15.0,
            spine_pitch_max: 0.0,
            smoothing: 10.0,
        }
    }
}

/// Smoothed gaze state stored on the VRM root entity.
/// Inserted automatically when `BodyTracking` is present.
#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component)]
pub struct SmoothedGaze {
    pub yaw: f32,
    pub pitch: f32,
}
