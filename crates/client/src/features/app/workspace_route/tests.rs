use super::{active_server_for_route, server_room_target};
use crate::Route;

#[test]
fn server_and_room_routes_select_their_server_in_rail() {
    let server = Route::AppServer {
        server_id: "server-a".to_owned(),
    };
    let room = Route::AppServerRoom {
        server_id: "server-b".to_owned(),
        room_id: "room-1".to_owned(),
    };

    assert_eq!(
        active_server_for_route(&server).as_deref(),
        Some("server-a")
    );
    assert_eq!(active_server_for_route(&room).as_deref(), Some("server-b"));
}

#[test]
fn social_and_host_routes_have_no_active_server_in_rail() {
    let routes = [
        Route::AppFriends {},
        Route::AppDirectMessage {
            conversation_id: "conversation-1".to_owned(),
        },
        Route::AppHostSettings {},
        Route::AppHostSystemSettings {},
    ];

    for route in routes {
        assert_eq!(active_server_for_route(&route), None, "{route:?}");
    }
}

#[test]
fn room_route_target_tracks_room_and_server_navigation() {
    let first = Route::AppServerRoom {
        server_id: "server-a".to_owned(),
        room_id: "room-1".to_owned(),
    };
    let second = Route::AppServerRoom {
        server_id: "server-a".to_owned(),
        room_id: "room-2".to_owned(),
    };
    let server_only = Route::AppServer {
        server_id: "server-a".to_owned(),
    };

    assert_eq!(
        server_room_target(&first, "server-a"),
        (true, Some("room-1".to_owned()))
    );
    assert_eq!(
        server_room_target(&second, "server-a"),
        (true, Some("room-2".to_owned()))
    );
    assert_eq!(server_room_target(&second, "server-b"), (false, None));
    assert_eq!(server_room_target(&server_only, "server-a"), (true, None));
}

#[test]
fn social_and_host_routes_do_not_activate_server_rooms() {
    assert_eq!(
        server_room_target(&Route::AppFriends {}, "server-a"),
        (false, None)
    );
    assert_eq!(
        server_room_target(&Route::AppHostSettings {}, "server-a"),
        (false, None)
    );
}
