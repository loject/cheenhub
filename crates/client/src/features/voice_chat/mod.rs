//! Voice chat client feature.

mod kicked_modal;
mod local_video;
mod participant_focus_strip;
mod participant_grid;
mod participant_grid_data;
mod participant_tile;
mod provider;
mod realtime;
mod room_presence;
mod sidebar_controls;
mod speaking;
mod state;
mod surface;
mod video_fragments;
mod video_streams;
mod voice_controls;

pub(crate) use provider::VoiceConnectionProvider;
pub(crate) use sidebar_controls::SidebarVoiceControls;
pub(crate) use state::{VoiceConnectionHandle, VoiceConnectionState, VoiceRoomTarget};
pub(crate) use surface::VoiceRoomSurface;
