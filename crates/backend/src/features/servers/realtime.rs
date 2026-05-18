//! Server management realtime adapter.

use cheenhub_contracts::realtime::{
    AssignServerMemberRole, KickServerInviteMember, KickServerMember, ListServerInvites,
    ListServerMembers, ListServerRoles, RealtimeEnvelope, RealtimeKind, RealtimeModule,
    RejectionCode, RevokeServerInvite, RevokeServerMemberRole, SaveServerRoles, ServerKind,
};
use uuid::Uuid;

use crate::features::servers::application;
use crate::features::servers::error::ServerError;
use crate::realtime::EnvelopeSink;
use crate::realtime::protocol::{
    decode_payload, require_request_id, send_rejection, write_envelope,
};
use crate::state::AppState;

/// Handles one server management module envelope.
pub(crate) async fn handle(
    state: &AppState,
    user_id: &Uuid,
    send: &EnvelopeSink,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    match envelope.kind {
        RealtimeKind::Server(ServerKind::ListServerMembers) => {
            let request_id = require_request_id(&envelope)?;
            let payload: ListServerMembers = decode_payload(&envelope)?;
            match application::list_server_members(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::Server,
                        RealtimeKind::Server(ServerKind::ServerMemberList),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_server_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::Server(ServerKind::ListServerInvites) => {
            let request_id = require_request_id(&envelope)?;
            let payload: ListServerInvites = decode_payload(&envelope)?;
            match application::list_server_invites(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::Server,
                        RealtimeKind::Server(ServerKind::ServerInviteList),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_server_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::Server(ServerKind::RevokeServerInvite) => {
            let request_id = require_request_id(&envelope)?;
            let payload: RevokeServerInvite = decode_payload(&envelope)?;
            match application::revoke_server_invite(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::Server,
                        RealtimeKind::Server(ServerKind::ServerInviteRevoked),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_server_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::Server(ServerKind::KickServerInviteMember) => {
            let request_id = require_request_id(&envelope)?;
            let payload: KickServerInviteMember = decode_payload(&envelope)?;
            match application::kick_server_invite_member(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::Server,
                        RealtimeKind::Server(ServerKind::ServerInviteMemberKicked),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_server_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::Server(ServerKind::KickServerMember) => {
            let request_id = require_request_id(&envelope)?;
            let payload: KickServerMember = decode_payload(&envelope)?;
            match application::kick_server_member(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::Server,
                        RealtimeKind::Server(ServerKind::ServerMemberKicked),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_server_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::Server(ServerKind::ListServerRoles) => {
            let request_id = require_request_id(&envelope)?;
            let payload: ListServerRoles = decode_payload(&envelope)?;
            match application::list_server_roles(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::Server,
                        RealtimeKind::Server(ServerKind::ServerRoleList),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_server_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::Server(ServerKind::SaveServerRoles) => {
            let request_id = require_request_id(&envelope)?;
            let payload: SaveServerRoles = decode_payload(&envelope)?;
            match application::save_server_roles(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::Server,
                        RealtimeKind::Server(ServerKind::ServerRolesSaved),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_server_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::Server(ServerKind::AssignServerMemberRole) => {
            let request_id = require_request_id(&envelope)?;
            let payload: AssignServerMemberRole = decode_payload(&envelope)?;
            match application::assign_server_member_role(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::Server,
                        RealtimeKind::Server(ServerKind::ServerMemberRoleAssigned),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_server_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::Server(ServerKind::RevokeServerMemberRole) => {
            let request_id = require_request_id(&envelope)?;
            let payload: RevokeServerMemberRole = decode_payload(&envelope)?;
            match application::revoke_server_member_role(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::Server,
                        RealtimeKind::Server(ServerKind::ServerMemberRoleRevoked),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_server_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::Server(_) => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::UnsupportedMessage,
                "Unsupported server management message.",
            )
            .await
        }
        _ => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::BadRequest,
                "Realtime kind does not belong to server module.",
            )
            .await
        }
    }
}

async fn reject_server_error(
    send: &EnvelopeSink,
    request_id: Option<Uuid>,
    error: ServerError,
) -> anyhow::Result<()> {
    match error {
        ServerError::BadRequest(message) => {
            send_rejection(send, request_id, RejectionCode::BadRequest, &message).await
        }
        ServerError::NotFound(message) => {
            send_rejection(send, request_id, RejectionCode::BadRequest, &message).await
        }
        ServerError::Unauthorized(message) => {
            send_rejection(send, request_id, RejectionCode::Unauthorized, &message).await
        }
        ServerError::Internal(error) => {
            tracing::error!(%error, "server management realtime request failed");
            send_rejection(
                send,
                request_id,
                RejectionCode::InternalError,
                "Не удалось выполнить действие сервера.",
            )
            .await
        }
    }
}
