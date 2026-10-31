//! Presentation shared by the graphical model viewer and solo game prototype.
#[cfg(feature = "viewer")]
pub mod scene;

#[cfg(feature = "viewer")]
pub mod hunter;

#[cfg(feature = "viewer")]
pub mod animation;

#[cfg(feature = "viewer")]
pub mod combat_ui;
#[cfg(feature = "viewer")]
pub mod inventory_ui;

pub mod asset_paths;
#[cfg(feature = "viewer")]
pub mod monster;
#[cfg(feature = "viewer")]
mod rig_geometry;
