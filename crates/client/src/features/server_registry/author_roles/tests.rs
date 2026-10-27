use std::collections::HashMap;

use super::*;

#[test]
fn page_hydration_is_scoped_to_the_requested_server_and_author_ids() {
    let mut cache = AuthorRoleCache::default();
    let generation = cache.generation("one");
    let authors = vec!["author".to_owned(), "departed".to_owned()];
    let roles = HashMap::from([
        (
            "author".to_owned(),
            vec!["custom".to_owned(), "member".to_owned()],
        ),
        ("unrequested".to_owned(), vec!["other".to_owned()]),
    ]);
    assert!(cache.absorb_page("one", generation, &authors, &roles));
    assert_eq!(
        cache.roles("one", "author"),
        Some(&["custom".to_owned(), "member".to_owned()][..])
    );
    assert_eq!(cache.roles("one", "departed"), Some(&[][..]));
    assert_eq!(cache.roles("one", "unrequested"), None);
    assert_eq!(cache.roles("two", "author"), None);
    assert_eq!(cache.unresolved("one", &authors), Vec::<String>::new());
}

#[test]
fn invalidation_prevents_old_page_from_reintroducing_stale_colors() {
    let mut cache = AuthorRoleCache::default();
    let generation = cache.generation("server");
    cache.set_user("server", "author", vec!["initial".to_owned()]);
    cache.invalidate_user("server", "author");
    assert_eq!(cache.roles("server", "author"), None);
    assert!(!cache.absorb_page(
        "server",
        generation,
        &["author".to_owned()],
        &HashMap::from([("author".to_owned(), vec!["initial".to_owned()])]),
    ));
    assert_eq!(cache.roles("server", "author"), None);
    cache.set_user("server", "author", vec!["updated".to_owned()]);
    assert_eq!(
        cache.roles("server", "author"),
        Some(&["updated".to_owned()][..])
    );
}

#[test]
fn cache_is_bounded_even_with_many_distinct_message_authors() {
    let mut cache = AuthorRoleCache::default();
    for id in 0..=CACHE_LIMIT {
        cache.set_user("server", &id.to_string(), vec!["member".to_owned()]);
    }
    assert_eq!(cache.len(), CACHE_LIMIT);
    assert_eq!(cache.roles("server", "0"), None);
    assert!(cache.roles("server", &CACHE_LIMIT.to_string()).is_some());
}
