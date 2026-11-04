use super::{Cache, MAX_BYTES, MAX_ENTRIES, parse_cached};
use cheenhub_client_worker::markdown::MessageNode;
use std::rc::Rc;

#[test]
fn unchanged_message_reuses_parsed_tree() {
    let first = parse_cached(Some("cache-test-a"), "**один**").unwrap();
    let second = parse_cached(Some("cache-test-a"), "**один**").unwrap();

    assert!(Rc::ptr_eq(&first, &second));
}

#[test]
fn changed_body_replaces_cached_tree() {
    let first = parse_cached(Some("cache-test-b"), "первый").unwrap();
    let second = parse_cached(Some("cache-test-b"), "второй").unwrap();

    assert!(!Rc::ptr_eq(&first, &second));
    assert_ne!(first, second);
}

#[test]
fn cache_evicts_least_recently_used_entry_at_capacity() {
    let mut cache = Cache::default();
    for index in 0..MAX_ENTRIES {
        cache.insert(&index.to_string(), "x", Rc::new(vec![]));
    }
    assert!(cache.get("0", "x").is_some());
    cache.insert("new", "x", Rc::new(vec![]));

    assert!(cache.get("1", "x").is_none());
    assert!(cache.get("0", "x").is_some());
    assert_eq!(cache.entries.len(), MAX_ENTRIES);
}

#[test]
fn different_messages_and_temporary_messages_do_not_share_entries() {
    let first = parse_cached(Some("distinct-a"), "текст").unwrap();
    let second = parse_cached(Some("distinct-b"), "текст").unwrap();
    let temporary_a = parse_cached(None, "текст").unwrap();
    let temporary_b = parse_cached(None, "текст").unwrap();

    assert!(!Rc::ptr_eq(&first, &second));
    assert!(!Rc::ptr_eq(&temporary_a, &temporary_b));
}

#[test]
fn memory_budget_evicts_entries_before_exceeding_limit() {
    let mut cache = Cache::default();
    let body = "x".repeat(60_000);
    for index in 0..100 {
        cache.insert(
            &index.to_string(),
            &body,
            Rc::new(vec![MessageNode::Text(body.clone())]),
        );
        assert!(cache.bytes <= MAX_BYTES);
    }
    assert!(cache.entries.len() < MAX_ENTRIES);
    assert!(cache.get("0", &body).is_none());
}

#[test]
fn parses_many_short_messages_and_reuses_long_message() {
    let start = std::time::Instant::now();
    for index in 0..1_000 {
        parse_cached(Some(&format!("bench-{index}")), "Привет **мир**").unwrap();
    }
    let short_elapsed = start.elapsed();
    let long = "**длинное сообщение** ".repeat(1_000);
    let start = std::time::Instant::now();
    let first = parse_cached(Some("bench-long"), &long).unwrap();
    let first_elapsed = start.elapsed();
    let start = std::time::Instant::now();
    for _ in 0..1_000 {
        let next = parse_cached(Some("bench-long"), &long).unwrap();
        assert!(Rc::ptr_eq(&first, &next));
    }
    eprintln!(
        "markdown debug timings: 1000 short={short_elapsed:?}, long={first_elapsed:?}, 1000 hits={:?}",
        start.elapsed()
    );
}
