//! Жизненный цикл GlobalShortcuts: автоматическое восстановление и явное назначение.

use super::Status;
use super::{Command, Service, binding, identity};
use ashpd::{
    desktop::{
        Session,
        global_shortcuts::{GlobalShortcuts, NewShortcut, Shortcut},
    },
    zbus,
};
use dioxus::prelude::{debug, info, warn};
use futures_util::{FutureExt, StreamExt};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

mod assignment;
mod plan;
mod signals;
use assignment::Assignment;
use plan::{Plan, plan};

/// UUID действия сохраняет системную привязку между запусками.
pub(super) const SHORTCUT_ID: &str = "3a94f9ae-e764-4dad-a59c-b0323847a66e";

/// Причина завершения одной сессии определяет восстановление без permission dialog.
pub(super) enum End {
    /// Следующая пользовательская команда.
    Command(Option<Command>),
    /// Транспорт потерян; повторяется только восстановление.
    Disconnected(String),
    /// Нет потребителей и незавершённого назначения.
    Idle,
    /// Restore не подтвердил сохранённое действие; требуется «Назначить».
    BindingRequired,
    /// Отзыв или ошибка требуют явного действия пользователя.
    Failed,
}

/// Проверяет backend через CreateSession; v1 достаточно для базового PTT.
///
/// # Errors
/// Возвращает причину ошибки Registry, отсутствия API или отказа CreateSession.
pub(super) async fn open() -> Result<(GlobalShortcuts, Session<GlobalShortcuts>), String> {
    let connection = identity::connect().await?;
    let portal = GlobalShortcuts::with_connection(connection)
        .await
        .map_err(|error| error.to_string())?;
    if portal.version() < 1 {
        return Err("GlobalShortcuts API unavailable".into());
    }
    let session = portal
        .create_session(Default::default())
        .await
        .map_err(|error| error.to_string())?;
    Ok((portal, session))
}

/// Владеет одной сессией и закрывает gate до каждого освобождения.
pub(super) async fn run(
    service: &Arc<Service>,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    reply: Option<oneshot::Sender<Result<binding::PushToTalkKey, String>>>,
) -> End {
    let mut assignment = Assignment::new(reply);
    let opened = tokio::time::timeout(Duration::from_secs(5), open()).await;
    let (portal, session) = match opened {
        Ok(Ok(opened)) => opened,
        result => {
            let detail = match result {
                Ok(Err(error)) => error,
                _ => "portal startup timeout".into(),
            };
            assignment.reject(service, "Не удалось подключить системное управление Push-to-talk. Проверьте установку приложения и повторите назначение.");
            return End::Disconnected(detail);
        }
    };
    service.available(true);
    info!(
        shortcut_id = SHORTCUT_ID,
        version = portal.version(),
        "Linux PTT portal session created"
    );
    let end = events(service, commands, &portal, &session, &mut assignment).await;
    service.state.fail();
    assignment.reject(
        service,
        "Системное назначение прервано. Сохранённое сочетание не изменено.",
    );
    if !matches!(
        tokio::time::timeout(Duration::from_secs(1), session.close()).await,
        Ok(Ok(()))
    ) {
        warn!(
            kind = "portal_close",
            "Linux PTT session close failed; connection will be released"
        );
    }
    info!("Linux PTT portal session released; gate closed");
    end
}

async fn events(
    service: &Service,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    portal: &GlobalShortcuts,
    session: &Session<GlobalShortcuts>,
    assignment: &mut Assignment,
) -> End {
    let result: Result<End, ashpd::Error> = async {
        let mut signals = portal.receive_all_signals().await?;
        let mut closed = session.receive_closed().await?;
        let mut owner = portal.receive_owner_changed().await?;
        let mut bus = zbus::MessageStream::from(portal.connection().clone());
        let path = serde_json::to_value(session).expect("session serialization");
        let path = path.as_str().expect("session object path");
        let saved = !binding::raw_label().is_empty();
        let assigning = assignment.active();
        let startup = async {
            let previous = tokio::time::timeout(Duration::from_secs(5), portal.list_shortcuts(session, Default::default()))
                .await.map_err(|_| ashpd::Error::NoResponse)??.response()?;
            let operation = plan(assigning, shortcut_label(previous.shortcuts()).is_some(), saved, portal.version());
            debug!(?operation, saved, assigning, version = portal.version(), kind = "portal_binding_plan", "Linux PTT registration operation selected");
            if !operation.requires_bind() { return Ok((operation, shortcut_label(previous.shortcuts()))); }
            let shortcuts = [NewShortcut::new(SHORTCUT_ID, "Говорить при удержании сочетания")];
            let result = portal.bind_shortcuts(session, &shortcuts, None, Default::default()).await?.response()?;
            Ok::<_, ashpd::Error>((operation, shortcut_label(result.shortcuts())))
        };
        tokio::pin!(startup);
        let (operation, mut label) = loop {
            tokio::select! {
                result = &mut startup => break result?,
                _ = assignment.cancelled(), if assigning => return Ok(End::Idle),
                _ = closed.next() => return Ok(revoked(service)),
                _ = owner.next() => return Ok(End::Disconnected("portal owner changed during bind".into())),
                message = bus.next() => if !matches!(message, Some(Ok(_))) { return Ok(End::Disconnected("D-Bus disconnected during bind".into())); },
                command = commands.recv() => match command {
                    Some(Command::Ensure | Command::Release) => {
                        if !assigning && service.users.load(Ordering::Acquire) == 0 { return Ok(End::Idle); }
                    },
                    Some(Command::FinishAssignment) => {},
                    command => return Ok(End::Command(command)),
                },
            }
        };
        if operation == Plan::BindingRequired {
            if let Some(label) = label.as_deref() { service.binding_changed(Some(label)); }
            return Ok(End::BindingRequired);
        }
        if operation == Plan::SettingsRequired {
            info!(kind = "portal_configure_unavailable", "Linux PTT v1 reassignment requires system settings; bind skipped");
            assignment.reject(service, "В этой среде повторное назначение доступно в системных настройках сочетаний. Измените там сочетание CheenHub.");
            return Ok(End::Idle);
        }
        drain_old_holds(service, &mut signals, path, &mut label)?;
        let Some(initial_label) = label.as_deref() else {
            service.binding_changed(None);
            assignment.reject(service, "Сочетание не назначено. Нажмите «Назначить», чтобы выбрать его.");
            return Ok(End::BindingRequired);
        };
        service.binding_changed(Some(initial_label));
        match operation {
            Plan::Reassign => {
                ping(portal).await?;
                drain_old_holds(service, &mut signals, path, &mut label)?;
                // Configure не имеет Request/Response и не сообщает системную отмену.
                tokio::time::timeout(Duration::from_secs(5), portal.configure_shortcuts(session, None, Default::default()))
                    .await.map_err(|_| ashpd::Error::NoResponse)??;
                service.configuring(true);
                debug!(kind = "portal_configure", "Linux PTT system configuration opened");
            },
            Plan::FirstAssignment | Plan::Restore => assignment.complete(service),
            Plan::BindingRequired | Plan::SettingsRequired => unreachable!(),
        }
        if !assignment.keeps_session(service.users.load(Ordering::Acquire)) { return Ok(End::Idle); }
        if !assignment.active() {
            ping(portal).await?;
            drain_old_holds(service, &mut signals, path, &mut label)?;
            service.binding_changed(label.as_deref());
            ready(service);
        }
        let health = heartbeat(service, portal);
        tokio::pin!(health);
        loop {
            tokio::select! {
                message = signals.next() => {
                    let Some(message) = message else { return Ok(End::Disconnected("portal signal stream ended".into())); };
                    match signals::decode(&message, path)? {
                        signals::Event::Held(held) => { if !assignment.active() { service.state.event(held); } },
                        signals::Event::Changed(next_label) => {
                            service.state.fail();
                            label = next_label;
                            ping(portal).await?;
                            drain_old_holds(service, &mut signals, path, &mut label)?;
                            if let Some(end) = changed(service, assignment, label.as_deref()) { return Ok(end); }
                        },
                        signals::Event::Ignored => {},
                    }
                },
                _ = assignment.cancelled(), if assignment.active() => {
                    assignment.cancel(service);
                    if service.users.load(Ordering::Acquire) == 0 { return Ok(End::Idle); }
                    ping(portal).await?;
                    drain_old_holds(service, &mut signals, path, &mut label)?;
                    service.binding_changed(label.as_deref());
                    ready(service);
                },
                _ = closed.next() => return Ok(revoked(service)),
                _ = owner.next() => return Ok(End::Disconnected("portal owner changed".into())),
                message = bus.next() => if !matches!(message, Some(Ok(_))) { return Ok(End::Disconnected("D-Bus disconnected".into())); },
                detail = &mut health => return Ok(End::Disconnected(detail)),
                command = commands.recv() => match command {
                    Some(Command::Ensure | Command::Release) => {
                        if !assignment.active() {
                            if service.users.load(Ordering::Acquire) == 0 { return Ok(End::Idle); }
                            service.state.fail();
                            ping(portal).await?;
                            drain_old_holds(service, &mut signals, path, &mut label)?;
                            service.binding_changed(label.as_deref());
                            ready(service);
                        }
                    },
                    Some(Command::FinishAssignment) => {
                        // Сначала обрабатываем изменения, уже доставленные до подтверждения UI.
                        ping(portal).await?;
                        drain_old_holds(service, &mut signals, path, &mut label)?;
                        service.binding_changed(label.as_deref());
                        assignment.complete(service);
                        if service.users.load(Ordering::Acquire) == 0 { return Ok(End::Idle); }
                        ready(service);
                    },
                    Some(Command::Assign(reply)) if portal.version() < 2 => {
                        Assignment::new(Some(reply)).reject(service, "В этой среде повторное назначение доступно в системных настройках сочетаний. Измените там сочетание CheenHub.");
                        service.state.fail();
                        ping(portal).await?;
                        drain_old_holds(service, &mut signals, path, &mut label)?;
                        ready(service);
                    },
                    command => return Ok(End::Command(command)),
                },
            }
        }
    }.await;
    match result {
        Ok(end) => end,
        Err(error) => {
            warn!(%error, kind = "portal_session", "Linux PTT portal operation failed; gate closed");
            if matches!(error, ashpd::Error::NoResponse) {
                service.state.fail();
                return End::Disconnected(error.to_string());
            }
            service.publish(Status::Failed("Системное управление Push-to-talk не ответило или отказало в доступе. Передача остановлена. Повторите назначение сочетания.".into()));
            End::Failed
        }
    }
}

fn ready(service: &Service) {
    service.state.recover();
    service.publish(Status::Ready);
}

fn changed(service: &Service, assignment: &mut Assignment, label: Option<&str>) -> Option<End> {
    service.state.fail();
    service.binding_changed(label);
    if label.is_none() {
        return Some(revoked(service));
    }
    assignment.complete(service);
    if service.users.load(Ordering::Acquire) == 0 {
        return Some(End::Idle);
    }
    ready(service);
    None
}

fn revoked(service: &Service) -> End {
    service.publish(Status::Failed("Доступ к Push-to-talk отозван или системная сессия закрыта. Передача остановлена. Нажмите «Назначить», чтобы восстановить доступ.".into()));
    End::Failed
}

/// Находит описание только нашего действия; пустой trigger не разрешает передачу.
pub(super) fn shortcut_label(shortcuts: &[Shortcut]) -> Option<String> {
    shortcuts
        .iter()
        .find(|shortcut| shortcut.id() == SHORTCUT_ID)
        .map(|shortcut| shortcut.trigger_description().to_owned())
        .filter(|label| !label.is_empty())
}

async fn heartbeat(service: &Service, portal: &GlobalShortcuts) -> String {
    loop {
        if ping(portal).await.is_err() {
            service.state.fail();
            return "portal watchdog failed".into();
        }
        service.state.heartbeat();
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

async fn ping(portal: &GlobalShortcuts) -> Result<(), ashpd::Error> {
    let ping = portal.connection().call_method(
        Some("org.freedesktop.portal.Desktop"),
        "/org/freedesktop/portal/desktop",
        Some("org.freedesktop.DBus.Peer"),
        "Ping",
        &(),
    );
    tokio::time::timeout(Duration::from_millis(500), ping)
        .await
        .map_err(|_| ashpd::Error::NoResponse)??;
    Ok(())
}

fn drain_old_holds(
    service: &Service,
    signals: &mut zbus::proxy::SignalStream<'static>,
    path: &str,
    label: &mut Option<String>,
) -> Result<(), ashpd::Error> {
    while let Some(message) = signals.next().now_or_never().flatten() {
        fold_pending(service, &message, path, label)?;
    }
    Ok(())
}

fn fold_pending(
    service: &Service,
    message: &zbus::Message,
    path: &str,
    label: &mut Option<String>,
) -> Result<(), ashpd::Error> {
    let result = signals::fold_pending(message, path, label);
    if result.is_err() && label.is_none() {
        service.binding_changed(None);
        revoked(service);
    }
    result
}

#[cfg(test)]
mod tests;
