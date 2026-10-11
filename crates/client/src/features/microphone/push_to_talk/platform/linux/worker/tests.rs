use super::{Command, capture_command_needs_session};

#[test]
fn delayed_release_reopens_session_for_new_capture() {
    // Старый capture отправил Release после Ensure нового capture.
    let queued_commands = [Command::Ensure, Command::Release];

    let reopen = queued_commands
        .iter()
        .map(|command| capture_command_needs_session(command, 1))
        .collect::<Vec<_>>();

    assert_eq!(reopen, [true, true]);
}

#[test]
fn capture_commands_do_not_open_session_without_users() {
    assert!(!capture_command_needs_session(&Command::Ensure, 0));
    assert!(!capture_command_needs_session(&Command::Release, 0));
}
