//! Проверки сохранения декодируемой последовательности до появления видеоплитки.

use super::*;

fn frame(sequence: u64, key_frame: bool, bytes: usize) -> ParticipantVideoFrame {
    ParticipantVideoFrame {
        room_id: "room".into(),
        sender_user_id: "peer".into(),
        sequence,
        timestamp_us: sequence * 33_333,
        duration_us: 33_333,
        bytes: vec![0; bytes],
        key_frame,
    }
}

#[test]
fn incomplete_startup_sequence_is_replaced_by_the_next_key_frame() {
    let mut delivery = ParticipantVideoDelivery::default();
    delivery.publish(frame(0, false, 1));
    delivery.publish(frame(1, true, 1));
    delivery.publish(frame(2, false, 1));
    delivery.publish(frame(3, true, 1));
    delivery.publish(frame(4, false, 1));

    let mut receiver = delivery.subscribe();

    assert_eq!(receiver.try_recv().unwrap(), frame(3, true, 1));
    assert_eq!(receiver.try_recv().unwrap(), frame(4, false, 1));
    assert!(receiver.try_recv().is_err());
}

#[test]
fn frame_limit_discards_the_incomplete_sequence_and_recovers_at_a_key_frame() {
    let mut delivery = ParticipantVideoDelivery::default();
    delivery.publish(frame(0, true, 1));
    for sequence in 1..=128 {
        delivery.publish(frame(sequence, false, 1));
    }

    let mut receiver = delivery.subscribe();
    assert!(receiver.try_recv().is_err());
    delivery.publish(frame(129, true, 1));

    assert_eq!(receiver.try_recv().unwrap(), frame(129, true, 1));
}

#[test]
fn byte_limit_discards_the_sequence_without_replaying_deltas_alone() {
    let mut delivery = ParticipantVideoDelivery::default();
    delivery.publish(frame(0, true, 8 * 1024 * 1024));
    delivery.publish(frame(1, false, 1));
    delivery.publish(frame(2, false, 1));

    let mut receiver = delivery.subscribe();

    assert!(receiver.try_recv().is_err());
}

#[test]
fn live_delivery_continues_after_draining_the_startup_buffer() {
    let mut delivery = ParticipantVideoDelivery::default();
    delivery.publish(frame(0, true, 1));
    let mut receiver = delivery.subscribe();
    assert_eq!(receiver.try_recv().unwrap(), frame(0, true, 1));

    delivery.publish(frame(1, false, 1));

    assert_eq!(receiver.try_recv().unwrap(), frame(1, false, 1));
    assert!(receiver.try_recv().is_err());
}
