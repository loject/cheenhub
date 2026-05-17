//! Mock server invite data for the settings UI.

/// Server invite availability status.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum InviteStatus {
    /// Invite can be used to join the server.
    Active,
    /// Invite is visible but cannot be used to join the server.
    Paused,
}

/// Server invite link shown in settings.
#[derive(Clone, PartialEq)]
pub(super) struct InviteLink {
    /// Stable local invite row id.
    pub(super) id: String,
    /// Invite code shown to administrators.
    pub(super) code: String,
    /// Display name of the invite creator.
    pub(super) author: &'static str,
    /// Human-readable creation time.
    pub(super) created_at: &'static str,
    /// Human-readable expiration time.
    pub(super) expires_at: &'static str,
    /// Optional usage limit.
    pub(super) max_uses: Option<u32>,
    /// Current invite status.
    pub(super) status: InviteStatus,
    /// Members who joined through this invite.
    pub(super) joined_members: Vec<InviteJoin>,
}

/// Member entry joined through an invite.
#[derive(Clone, PartialEq)]
pub(super) struct InviteJoin {
    /// Stable member id.
    pub(super) id: &'static str,
    /// Member display name.
    pub(super) name: &'static str,
    /// Human-readable join time.
    pub(super) joined_at: &'static str,
}

/// Returns mock invite links for the UI-only server settings section.
pub(super) fn mock_invites() -> Vec<InviteLink> {
    vec![
        InviteLink {
            id: "daily-standup".to_owned(),
            code: "4e63a2ba-5d6f-4f03-9d46-a7c536e1b8a1".to_owned(),
            author: "loject",
            created_at: "сегодня",
            expires_at: "через 7 дней",
            max_uses: Some(25),
            status: InviteStatus::Active,
            joined_members: vec![
                InviteJoin {
                    id: "a3f928d2-1f7a-48d1-a6b6-082a6ef83455",
                    name: "Mira",
                    joined_at: "сегодня, 14:20",
                },
                InviteJoin {
                    id: "7c5a72b4-b0f7-4f5d-b1de-174e17f481c6",
                    name: "Anton",
                    joined_at: "сегодня, 13:48",
                },
                InviteJoin {
                    id: "19d6f1b4-67be-48e7-b7ff-f3e4fc0f6a7f",
                    name: "Nika",
                    joined_at: "вчера, 21:12",
                },
            ],
        },
        InviteLink {
            id: "friends".to_owned(),
            code: "b0c7582d-90c7-4f3e-a936-6c21a6f688d4".to_owned(),
            author: "loject",
            created_at: "вчера",
            expires_at: "без срока",
            max_uses: None,
            status: InviteStatus::Active,
            joined_members: vec![
                InviteJoin {
                    id: "d0e25d1f-2f7d-4570-97b7-f9ef1d1dc8df",
                    name: "Kirill",
                    joined_at: "вчера, 18:04",
                },
                InviteJoin {
                    id: "85eb5e9b-a14f-4ad3-9e5a-3e671684d963",
                    name: "Lena",
                    joined_at: "2 дня назад, 10:31",
                },
                InviteJoin {
                    id: "a58f33f6-af63-44ff-a834-691df7bf10de",
                    name: "Sasha",
                    joined_at: "2 дня назад, 09:52",
                },
                InviteJoin {
                    id: "d96e1b2d-59f7-44ee-a16c-f41a4cab6263",
                    name: "Vlad",
                    joined_at: "3 дня назад, 22:18",
                },
                InviteJoin {
                    id: "63ec9968-2526-4215-8c0f-fc5e1626b104",
                    name: "Roma",
                    joined_at: "4 дня назад, 15:37",
                },
                InviteJoin {
                    id: "f1e95411-e8e8-48de-996f-49e8cf40dd93",
                    name: "Alina",
                    joined_at: "4 дня назад, 13:08",
                },
                InviteJoin {
                    id: "203d320b-0c26-4b6f-99c3-4d0d4f610d57",
                    name: "Denis",
                    joined_at: "5 дней назад, 20:44",
                },
                InviteJoin {
                    id: "c52ce4f0-5e8b-40dc-9d3a-33d076c1ab67",
                    name: "Marina",
                    joined_at: "5 дней назад, 12:16",
                },
                InviteJoin {
                    id: "24d2ea12-ee4e-42c0-b409-07eb10ba4680",
                    name: "Ilya",
                    joined_at: "6 дней назад, 17:22",
                },
                InviteJoin {
                    id: "66dd3392-352f-479a-8de2-6af0283198fe",
                    name: "Eva",
                    joined_at: "6 дней назад, 11:05",
                },
            ],
        },
        InviteLink {
            id: "event-room".to_owned(),
            code: "2a942f16-c24b-4ab4-a623-05a8f16a6e93".to_owned(),
            author: "moderator",
            created_at: "4 дня назад",
            expires_at: "через 2 дня",
            max_uses: Some(12),
            status: InviteStatus::Paused,
            joined_members: Vec::new(),
        },
    ]
}
