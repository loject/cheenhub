use cheenhub_contracts::rest::{SearchUsersResponse, UserRelationStatus};

use super::*;

#[tokio::test]
async fn search_reports_relation_for_every_candidate_in_one_response() {
    let setup = setup_pair().await;
    let charlie =
        registered_user(&setup.state, "charlie_search", "charlie-search@example.com").await;
    let dave = registered_user(&setup.state, "dave_search", "dave-search@example.com").await;
    let erin = registered_user(&setup.state, "erin_search", "erin-search@example.com").await;

    // Заявка от текущего пользователя: исходящая.
    send_friend_request(
        &setup.state,
        &setup.alice_access_token,
        SendFriendRequestRequest {
            recipient_user_id: dave.user.id.clone(),
        },
    )
    .await
    .expect("outgoing request should send");
    // Входящая заявка: другой пользователь отправил заявку текущему.
    send_friend_request(
        &setup.state,
        &charlie.access_token,
        SendFriendRequestRequest {
            recipient_user_id: setup.alice_user_id.clone(),
        },
    )
    .await
    .expect("incoming request should send");
    // У erin нет никаких связей с текущим пользователем.

    let response = search(&setup, "search").await;
    let relations = response
        .users
        .iter()
        .map(|user| (user.id.as_str(), user.relation))
        .collect::<Vec<_>>();

    assert_eq!(relations.len(), 3);
    assert_eq!(
        relation_of(&relations, &charlie.user.id),
        Some(UserRelationStatus::PendingIncoming)
    );
    assert_eq!(
        relation_of(&relations, &dave.user.id),
        Some(UserRelationStatus::PendingOutgoing)
    );
    assert_eq!(relation_of(&relations, &erin.user.id), None);
    // Принятая дружба из setup_pair тоже должна попасть в выдачу, если ник попал в поиск.
    assert!(relations.iter().all(|(id, _)| *id != setup.alice_user_id));
}

#[tokio::test]
async fn search_returns_no_relations_for_query_shorter_than_two_characters() {
    let setup = setup_pair().await;

    let response = search_users(
        &setup.state,
        &setup.alice_access_token,
        Some("a".to_owned()),
    )
    .await
    .expect("short query should not fail");

    assert!(response.users.is_empty());
}

async fn search(setup: &PairSetup, query: &str) -> SearchUsersResponse {
    search_users(
        &setup.state,
        &setup.alice_access_token,
        Some(query.to_owned()),
    )
    .await
    .expect("search should load")
}

fn relation_of(
    relations: &[(&str, Option<UserRelationStatus>)],
    user_id: &str,
) -> Option<UserRelationStatus> {
    relations
        .iter()
        .find(|(id, _)| *id == user_id)
        .map(|(_, relation)| *relation)
        .expect("user should be present in search results")
}
