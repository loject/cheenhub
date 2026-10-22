use super::*;

#[test]
fn parses_process_names_with_spaces_and_parentheses() {
    let mut fields = vec!["0"; 20];
    fields[0] = "S";
    fields[11] = "20";
    fields[12] = "7";
    fields[19] = "1234";

    let stat = format!("42 (worker (cpu)) {}", fields.join(" "));
    let (name, ticks, started_at) = parse_stat(&stat).unwrap();

    assert_eq!(name, "worker (cpu)");
    assert_eq!(ticks, 27);
    assert_eq!(started_at, 1234);
}

#[test]
fn excludes_shared_resident_pages_from_process_memory() {
    assert_eq!(private_memory("100 20 5 0 0 0 0", 4096).unwrap(), 61_440);
    assert_eq!(private_memory("100 5 20", 4096).unwrap(), 0);
}

#[test]
fn rejects_incomplete_process_counters() {
    assert!(parse_stat("42 (backend) S 0").is_err());
    assert!(private_memory("100 20", 4096).is_err());
}
