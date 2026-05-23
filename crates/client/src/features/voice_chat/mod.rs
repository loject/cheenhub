//! Voice chat client feature.

mod kicked_modal;
mod participant_grid;
mod participant_tile;
mod provider;
mod realtime;
mod screen_fragments;
mod screen_video;
mod sidebar_controls;
mod state;
mod surface;
mod voice_controls;

pub(crate) use provider::VoiceConnectionProvider;
pub(crate) use sidebar_controls::SidebarVoiceControls;
pub(crate) use state::{VoiceConnectionHandle, VoiceConnectionState, VoiceRoomTarget};
pub(crate) use surface::VoiceRoomSurface;
