#[cfg(feature = "free_camera")]
pub mod free;
pub mod tracking;

pub mod prelude {
    pub use bevy::prelude::*;
    pub use tiny_bail::prelude::*;

    #[cfg(feature = "free_camera")]
    pub use crate::free::{FreeCameraInput, FreeCameraInputPlugin};
    pub use crate::tracking::{
        SpringArm, SpringArmCameraPlugin, SpringArmCameraSettings, SpringArmCollisionFilter,
    };
}
