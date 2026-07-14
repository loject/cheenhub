//! Voice chat client feature.

mod kicked_modal;
mod local_video;
mod microphone_uplink;
mod microphone_uplink_platform;
mod notification_sounds;
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
mod voice_frame_sender;

pub(crate) use participant_grid::{VoiceParticipantGrid, VoiceParticipantGridStatus};
pub(crate) use provider::VoiceConnectionProvider;
pub(crate) use sidebar_controls::SidebarVoiceControls;
pub(crate) use state::{VoiceConnectionHandle, VoiceConnectionState, VoiceRoomTarget};
pub(crate) use surface::VoiceRoomSurface;
pub(crate) use voice_controls::VoiceControls;
