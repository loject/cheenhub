//! Единственный worker PTT; восстановление транспорта не восстанавливает удержание.

use super::{
    Command, Service,
    session::{self, End},
};
use crate::features::microphone::push_to_talk::Status;
use dioxus::prelude::{info, warn};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::sync::mpsc;

/// Проверяет реальный backend и обслуживает команды до остановки provider.
pub(super) async fn run(service: Arc<Service>, mut commands: mpsc::UnboundedReceiver<Command>) {
    probe(&service).await;
    let mut next = commands.recv().await;
    loop {
        let reply = match next {
            Some(Command::Assign(reply)) => Some(reply),
            Some(Command::Probe) => {
                probe(&service).await;
                next = if service.users.load(Ordering::Acquire) > 0 {
                    Some(Command::Ensure)
                } else {
                    commands.recv().await
                };
                continue;
            }
            Some(ref command)
                if capture_command_needs_session(
                    command,
                    service.users.load(Ordering::Acquire),
                ) =>
            {
                None
            }
            Some(Command::Ensure | Command::Release | Command::FinishAssignment) => {
                next = commands.recv().await;
                continue;
            }
            Some(Command::Shutdown) | None => break,
        };
        service.state.fail();
        match session::run(&service, &mut commands, reply).await {
            End::Command(command) => next = command,
            End::Idle => {
                service.publish(Status::Available);
                next = if service.users.load(Ordering::Acquire) > 0 {
                    Some(Command::Ensure)
                } else {
                    commands.recv().await
                };
            }
            End::BindingRequired => {
                service.publish(Status::BindingRequired);
                next = commands.recv().await;
            }
            End::Failed => {
                service.state.fail();
                warn!(
                    kind = "portal_revoked",
                    "Linux PTT session failed; user action required"
                );
                next = commands.recv().await;
            }
            End::Disconnected(detail) => {
                service.publish(Status::Failed("Соединение с системным управлением Push-to-talk потеряно. Передача остановлена; восстанавливаем подключение.".into()));
                warn!(%detail, kind = "portal_reconnect", "Linux PTT transport failed; gate closed");
                next = tokio::select! {
                    command = commands.recv() => command,
                    _ = tokio::time::sleep(Duration::from_secs(2)), if service.users.load(Ordering::Acquire) > 0 => Some(Command::Ensure),
                };
            }
        }
    }
    service.state.fail();
    info!("Linux PTT worker stopped");
}

fn capture_command_needs_session(command: &Command, users: usize) -> bool {
    // Запоздалый Release старого capture должен восстановить сессию нового capture.
    users > 0 && matches!(command, Command::Ensure | Command::Release)
}

#[cfg(test)]
mod tests;

async fn probe(service: &Service) {
    info!(
        kind = "portal_probe",
        "Linux PTT capability check pending; gate closed"
    );
    service.state.fail();
    service.available.store(false, Ordering::Release);
    service
        .updates
        .send_modify(|update| update.availability = super::Availability::Pending);
    let result = tokio::time::timeout(Duration::from_secs(4), session::open()).await;
    match result {
        Ok(Ok((_, session))) => {
            let _ = tokio::time::timeout(Duration::from_secs(1), session.close()).await;
            service.available(true);
            service.publish(Status::Available);
            info!(
                kind = "portal_available",
                "Linux PTT GlobalShortcuts backend verified"
            );
        }
        result => {
            warn!(
                ?result,
                kind = "portal_unavailable",
                "Linux PTT GlobalShortcuts backend unavailable"
            );
            service.available(false);
            service.publish(Status::Failed("Системное управление Push-to-talk недоступно. Проверьте установку приложения и поддержку глобальных сочетаний в вашей среде рабочего стола.".into()));
        }
    }
}
