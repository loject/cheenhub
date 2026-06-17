//! Рендерер аватара пользователя.

use dioxus::prelude::*;

struct AvatarPalette {
    background: &'static str,
    foreground: &'static str,
    outline_color: &'static str,
}

/// Seed, который аватары используют для выбора детерминированной палитры по умолчанию.
#[derive(Clone, Copy)]
pub(crate) struct AvatarSeed(Signal<String>);

const DEFAULT_AVATAR_PALETTES: &[AvatarPalette] = &[
    AvatarPalette {
        background: "linear-gradient(135deg, #1d4ed8 0%, #38bdf8 100%)",
        foreground: "#eff6ff",
        outline_color: "rgba(191, 219, 254, .34)",
    },
    AvatarPalette {
        background: "linear-gradient(135deg, #047857 0%, #5eead4 100%)",
        foreground: "#ecfdf5",
        outline_color: "rgba(167, 243, 208, .34)",
    },
    AvatarPalette {
        background: "linear-gradient(135deg, #be123c 0%, #fb7185 100%)",
        foreground: "#fff1f2",
        outline_color: "rgba(254, 205, 211, .34)",
    },
    AvatarPalette {
        background: "linear-gradient(135deg, #7c2d12 0%, #fbbf24 100%)",
        foreground: "#fffbeb",
        outline_color: "rgba(253, 230, 138, .34)",
    },
    AvatarPalette {
        background: "linear-gradient(135deg, #581c87 0%, #c084fc 100%)",
        foreground: "#faf5ff",
        outline_color: "rgba(233, 213, 255, .34)",
    },
    AvatarPalette {
        background: "linear-gradient(135deg, #0f766e 0%, #a3e635 100%)",
        foreground: "#f7fee7",
        outline_color: "rgba(217, 249, 157, .34)",
    },
    AvatarPalette {
        background: "linear-gradient(135deg, #4338ca 0%, #f472b6 100%)",
        foreground: "#fdf2f8",
        outline_color: "rgba(251, 207, 232, .34)",
    },
    AvatarPalette {
        background: "linear-gradient(135deg, #334155 0%, #94a3b8 100%)",
        foreground: "#f8fafc",
        outline_color: "rgba(226, 232, 240, .32)",
    },
];

/// Предоставляет детерминированный seed аватара по умолчанию для дочерних аватаров.
pub(crate) fn use_avatar_seed(seed: String) {
    let initial_seed = seed.clone();
    let mut avatar_seed = use_context_provider(move || AvatarSeed(Signal::new(initial_seed)));
    avatar_seed.0.set(seed);
}

/// Рендерит изображение аватара пользователя с подстановкой первой буквы ника.
#[component]
pub(crate) fn UserAvatar(
    nickname: String,
    avatar_url: Option<String>,
    class: String,
    avatar_seed: Option<String>,
) -> Element {
    let mut image_failed = use_signal(|| false);
    let show_image = avatar_url.is_some() && !image_failed();
    let avatar_seed = avatar_seed
        .or_else(|| try_consume_context::<AvatarSeed>().map(|seed| (seed.0)()))
        .unwrap_or_else(|| nickname.clone());
    let palette = default_avatar_palette(&avatar_seed);
    let fallback_style = format!(
        "background: {}; box-shadow: inset 0 0 0 1px {}; color: {};",
        palette.background, palette.outline_color, palette.foreground
    );

    rsx! {
        div { class: "{class} relative overflow-hidden",
            if show_image {
                img {
                    class: "absolute inset-0 h-full w-full object-cover",
                    src: avatar_url.unwrap_or_default(),
                    alt: "{nickname}",
                    onerror: move |_| image_failed.set(true),
                }
            } else {
                div {
                    class: "absolute inset-0 flex items-center justify-center",
                    style: "{fallback_style}",
                    role: "img",
                    "aria-label": "Аватар {nickname}",
                    svg {
                        class: "block",
                        style: "width: 68%; height: 68%;",
                        view_box: "503.703636 225.558456 544.640644 544.640644",
                        "aria-hidden": "true",
                        g { transform: "translate(0.000000,1024.000000) scale(0.100000,-0.100000)", fill: "currentColor", stroke: "none",
                            path { d: "M7660 7599 c-430 -44 -871 -208 -1211 -449 -504 -359 -708 -712 -726 -1255 -10 -312 51 -585 197 -874 253 -506 693 -889 1261 -1101 74 -28 139 -50 144 -50 6 0 -34 24 -88 53 -427 227 -774 665 -886 1117 -63 257 -72 551 -25 790 129 647 541 1187 1129 1479 274 136 521 200 811 208 150 5 295 -4 384 -22 l25 -6 -25 12 c-50 22 -279 70 -428 89 -163 21 -402 25 -562 9z" }
                            path { d: "M8245 7464 c-331 -35 -658 -154 -936 -340 -405 -270 -708 -678 -839 -1128 -53 -182 -88 -455 -74 -576 l7 -65 13 90 c75 514 405 956 901 1205 238 119 438 169 713 177 157 5 204 2 315 -15 295 -48 550 -155 767 -322 42 -33 80 -60 83 -60 10 0 535 422 535 430 0 11 -109 110 -214 195 -121 97 -243 173 -401 250 -231 113 -414 157 -665 160 -91 2 -183 1 -205 -1z" }
                            path { d: "M8249 6386 c-69 -28 -105 -53 -154 -107 -74 -82 -106 -203 -82 -309 9 -36 15 -65 14 -66 -10 -4 -224 -138 -410 -255 l-232 -146 -35 30 c-96 84 -253 117 -369 77 -148 -50 -235 -150 -263 -301 -12 -68 1 -148 36 -224 104 -229 420 -287 609 -112 l38 35 197 -95 c108 -52 256 -123 328 -158 l130 -63 0 -95 c-1 -89 1 -98 37 -170 42 -85 90 -132 177 -175 50 -24 65 -27 166 -27 107 0 113 1 180 35 128 65 198 187 198 340 -1 166 -117 311 -284 353 l-55 14 -6 224 c-3 123 -8 280 -10 348 l-3 124 45 11 c101 26 188 102 237 206 22 47 26 69 26 155 1 90 -2 106 -28 159 -41 84 -118 157 -200 189 -89 36 -204 37 -287 3z m-80 -660 c28 -19 64 -37 80 -41 27 -6 30 -10 35 -58 3 -29 8 -194 11 -368 l6 -315 -40 -18 c-21 -10 -54 -33 -72 -52 -18 -19 -39 -34 -46 -34 -7 0 -124 55 -260 123 -137 68 -284 140 -328 162 l-80 38 0 98 1 98 94 59 c102 63 367 230 477 300 37 23 68 42 69 42 1 0 25 -15 53 -34z" }
                            path { d: "M5640 6178 c-85 -191 -158 -470 -179 -692 -65 -661 174 -1332 639 -1795 174 -173 354 -286 575 -360 173 -57 410 -89 585 -77 531 35 1069 243 1536 593 62 46 117 89 122 96 6 7 -46 -15 -116 -50 -274 -135 -556 -196 -892 -195 -836 2 -1663 497 -2054 1227 -84 157 -157 354 -190 510 -49 237 -49 557 0 749 8 32 13 60 11 62 -2 3 -19 -28 -37 -68z" }
                            path { d: "M9416 4345 c-97 -154 -304 -372 -497 -521 -517 -403 -1045 -609 -1614 -631 -223 -8 -459 25 -659 93 -38 13 -72 24 -75 24 -17 -1 122 -81 244 -141 245 -119 456 -187 735 -236 150 -26 604 -26 770 1 659 104 1227 436 1603 935 65 87 147 213 147 227 0 3 -28 18 -62 33 -35 16 -135 64 -223 106 -205 99 -325 155 -333 155 -4 0 -20 -20 -36 -45z" }
                        }
                    }
                }
            }
        }
    }
}

fn default_avatar_palette(user_id: &str) -> &'static AvatarPalette {
    let index = stable_hash(user_id) % DEFAULT_AVATAR_PALETTES.len();
    &DEFAULT_AVATAR_PALETTES[index]
}

fn stable_hash(value: &str) -> usize {
    value.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        hash.wrapping_mul(0x100000001b3) ^ u64::from(byte)
    }) as usize
}
