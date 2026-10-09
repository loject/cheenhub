use super::*;

#[test]
fn holding_opens_gate_and_release_closes_it() {
    let mut gate = Gate::default();
    gate.recover(false);

    gate.event(true);
    assert!(gate.active());
    gate.event(false);
    assert!(!gate.active());
}

#[test]
fn failure_closes_gate_until_monitor_recovers() {
    let mut gate = Gate::default();
    gate.recover(true);
    assert!(gate.active());

    gate.failed();
    gate.event(true);
    assert!(!gate.active());

    gate.recover(false);
    assert!(!gate.active());
    gate.event(true);
    assert!(gate.active());
}
