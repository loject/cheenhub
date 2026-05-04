//! Landing page feature for the CheenHub web client.

use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
struct Feature {
    title: &'static str,
    description: &'static str,
    icon: FeatureIcon,
}

#[derive(Clone, Copy, PartialEq)]
enum FeatureIcon {
    Phone,
    Users,
    Screen,
    Code,
    Shield,
    CheckShield,
}

#[derive(Clone, Copy, PartialEq)]
struct TechItem {
    code: &'static str,
    label: &'static str,
}

#[derive(Clone, Copy, PartialEq)]
struct TechGroup {
    title: &'static str,
    items: &'static [TechItem],
}

const FEATURES: &[Feature] = &[
    Feature {
        title: "Низкая задержка",
        description: "WebTransport + Opus с адаптивным битрейтом. Около 1 мс в среднем по LAN.",
        icon: FeatureIcon::Phone,
    },
    Feature {
        title: "До 100 человек в комнате",
        description: "Всегда SFU-релей. Стабильное качество и предсказуемая работа при любой нагрузке.",
        icon: FeatureIcon::Users,
    },
    Feature {
        title: "Демонстрация экрана",
        description: "Захват окна или всего экрана. Отдельный трек — не влияет на голосовой поток.",
        icon: FeatureIcon::Screen,
    },
    Feature {
        title: "Самостоятельный хостинг",
        description: "Docker Compose, один файл. Разверни сам — твои данные на твоём сервере.",
        icon: FeatureIcon::Code,
    },
    Feature {
        title: "Нет трекинга",
        description: "Никакой телеметрии, рекламы или продажи метаданных. Открытый код — проверяй сам.",
        icon: FeatureIcon::Shield,
    },
    Feature {
        title: "Лицензия MIT",
        description: "Форки, коммерческое использование и интеграции — без лишних ограничений.",
        icon: FeatureIcon::CheckShield,
    },
];

const CLIENT_TECH: &[TechItem] = &[
    TechItem {
        code: "rs",
        label: "Rust",
    },
    TechItem {
        code: "dx",
        label: "Dioxus",
    },
    TechItem {
        code: "sg",
        label: "Signals",
    },
];

const SERVER_TECH: &[TechItem] = &[
    TechItem {
        code: "rs",
        label: "Rust",
    },
    TechItem {
        code: "WT",
        label: "WebTransport",
    },
    TechItem {
        code: "pg",
        label: "PostgreSQL",
    },
];

const MEDIA_TECH: &[TechItem] = &[
    TechItem {
        code: "WT",
        label: "Быстрый WebTransport",
    },
    TechItem {
        code: "au",
        label: "Кодек Opus",
    },
    TechItem {
        code: "SF",
        label: "SFU-релей",
    },
];

const INFRA_TECH: &[TechItem] = &[
    TechItem {
        code: "dk",
        label: "Docker",
    },
    TechItem {
        code: "gh",
        label: "GitHub Actions",
    },
    TechItem {
        code: "sh",
        label: "Самостоятельный хостинг",
    },
];

const TECH_GROUPS: &[TechGroup] = &[
    TechGroup {
        title: "Клиент",
        items: CLIENT_TECH,
    },
    TechGroup {
        title: "Сервер",
        items: SERVER_TECH,
    },
    TechGroup {
        title: "Медиа",
        items: MEDIA_TECH,
    },
    TechGroup {
        title: "Инфраструктура",
        items: INFRA_TECH,
    },
];

/// Renders the CheenHub landing page.
#[component]
pub(crate) fn LandingPage() -> Element {
    rsx! {
        div { class: "min-h-screen bg-zinc-950 text-zinc-100 selection:bg-zinc-700/40",
            div { class: "grid-bg min-h-screen",
                Header {}
                Hero {}
                FeaturesSection {}
                ComparisonSection {}
                TechSection {}
                CtaSection {}
                Footer {}
            }
        }
    }
}

#[component]
fn Header() -> Element {
    rsx! {
        nav { class: "sticky top-0 z-50 border-b border-zinc-800/80 bg-zinc-950/85 backdrop-blur-xl",
            div { class: "mx-auto flex max-w-6xl items-center justify-between px-5 py-3 lg:px-8",
                div { class: "flex items-center gap-3",
                    div { class: "flex h-8 w-8 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-100 text-zinc-950",
                        LogoIcon { class_name: "h-5 w-5" }
                    }
                    span { class: "text-sm font-semibold tracking-[-0.03em] text-zinc-50", "CheenHub" }
                    span { class: "hidden rounded-full border border-zinc-800 bg-zinc-900/80 px-2 py-0.5 text-[10px] uppercase tracking-[0.2em] text-zinc-500 sm:inline-flex", "Бета" }
                }
                div { class: "hidden items-center gap-0.5 sm:flex",
                    a { href: "#features", class: "rounded-lg px-3 py-1.5 text-[13px] text-zinc-400 transition hover:text-zinc-100", "Возможности" }
                    a { href: "#why", class: "rounded-lg px-3 py-1.5 text-[13px] text-zinc-400 transition hover:text-zinc-100", "Сравнение" }
                    a { href: "#tech", class: "rounded-lg px-3 py-1.5 text-[13px] text-zinc-400 transition hover:text-zinc-100", "Стек" }
                }
                div { class: "flex items-center gap-2",
                    a {
                        href: "https://github.com/loject/cheenhub",
                        target: "_blank",
                        rel: "noopener",
                        class: "btn-g hidden items-center rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[12px] font-medium text-zinc-300 sm:flex",
                        "GitHub"
                    }
                    a {
                        href: "https://youtube.com/@cheenhub",
                        target: "_blank",
                        rel: "noopener",
                        class: "btn-g hidden items-center rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[12px] font-medium text-zinc-300 sm:flex",
                        "YouTube"
                    }
                    TodoButton { class_name: "btn-g hidden items-center rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[12px] font-medium text-zinc-300 md:flex", label: "RuTube" }
                    TodoButton { class_name: "btn-g hidden items-center rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-[12px] font-medium text-zinc-300 md:flex", label: "Telegram" }
                }
            }
        }
    }
}

#[component]
fn Hero() -> Element {
    rsx! {
        section { class: "relative mx-auto max-w-6xl px-5 pb-16 pt-20 lg:px-8 lg:pt-28",
            div { class: "pointer-events-none absolute left-1/2 top-0 h-[500px] w-[800px] -translate-x-1/2 rounded-full bg-accent/5 blur-3xl" }
            div { class: "relative",
                div { class: "a1 mb-5 flex justify-center",
                    div { class: "inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/80 px-3.5 py-1.5 text-[11px] uppercase tracking-[0.22em] text-zinc-400",
                        span { class: "relative flex h-2 w-2",
                            span { class: "glow-ring" }
                            span { class: "relative h-2 w-2 rounded-full bg-accent/80" }
                        }
                        "Открытый код  ·  v0.9 бета  ·  бесплатно"
                    }
                }
                h1 { class: "a2 mx-auto max-w-3xl text-center text-4xl font-semibold leading-[1.1] tracking-[-0.05em] text-zinc-50 sm:text-5xl lg:text-[60px]",
                    "Голосовой чат,"
                    br {}
                    span { class: "text-zinc-400", "который просто работает." }
                }
                p { class: "a3 mx-auto mt-5 max-w-xl text-center text-[15px] leading-relaxed text-zinc-500",
                    "Открытая альтернатива Discord для геймеров — без лагов, без раздутости,"
                    br { class: "hidden sm:inline" }
                    "с разработкой в открытом эфире."
                }
                div { class: "a4 relative z-[30] mt-8 flex flex-wrap items-center justify-center gap-3",
                    WebButton { large: false }
                    DownloadDropdown { opens_up: false, large: false }
                }
                SocialLinks { class_name: "a5 mt-5 flex flex-wrap items-center justify-center gap-4 text-[13px] text-zinc-500", hover_class: "transition hover:text-zinc-200" }
                div { class: "a5 relative z-[10] mt-8 flex flex-wrap items-center justify-center gap-2",
                    Pill { strong: "< 1 мс", text: "задержка" }
                    Pill { strong: "Opus", text: "кодек" }
                    Pill { strong: "Быстрый", text: "WebTransport" }
                    Pill { strong: "MIT", text: "лицензия" }
                    div { class: "flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/80 px-3.5 py-1.5 text-[11px] text-zinc-400", "Нет трекинга" }
                }
            }
        }
    }
}

#[component]
fn FeaturesSection() -> Element {
    rsx! {
        section { id: "features", class: "mx-auto max-w-6xl px-5 pb-20 lg:px-8",
            SectionIntro {
                eyebrow: "Возможности",
                title: "Сделано правильно.",
                description: "Не очередной клон — конкретные решения конкретных проблем."
            }
            div { class: "grid gap-3 sm:grid-cols-2 lg:grid-cols-3",
                for feature in FEATURES {
                    FeatureCard { feature: *feature }
                }
            }
        }
    }
}

#[component]
fn ComparisonSection() -> Element {
    rsx! {
        section { id: "why", class: "mx-auto max-w-6xl px-5 pb-20 lg:px-8",
            div { class: "rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-6 shadow-[0_0_0_1px_rgba(255,255,255,0.02)]",
                div { class: "mb-6",
                    Eyebrow { label: "Сравнение", dark: true }
                    h2 { class: "text-xl font-semibold tracking-[-0.04em] text-zinc-50", "CheenHub и Discord" }
                    p { class: "mt-1 text-[13px] text-zinc-500", "Не атака. Просто честный список." }
                }
                div { class: "overflow-x-auto",
                    table { class: "w-full min-w-[540px]",
                        thead {
                            tr { class: "border-b border-zinc-800",
                                th { class: "w-1/3 pb-3 pr-4 text-left text-[11px] font-medium uppercase tracking-[0.2em] text-zinc-500", "Параметр" }
                                th { class: "pb-3 pr-4 text-left text-[11px] font-medium uppercase tracking-[0.2em] text-zinc-300", "CheenHub" }
                                th { class: "pb-3 text-left text-[11px] font-medium uppercase tracking-[0.2em] text-zinc-600", "Discord" }
                            }
                        }
                        tbody { class: "divide-y divide-zinc-800/50",
                            tr {
                                td { class: "py-3 pr-4 text-[13px] text-zinc-400", "Задержка голоса" }
                                td { class: "py-3 pr-4 text-[13px] text-zinc-200", "~1 мс" }
                                td { class: "py-3 text-[13px] text-zinc-600", "~60–120 мс через серверный релей" }
                            }
                            tr {
                                td { class: "py-3 pr-4 text-[13px] text-zinc-400", "Открытый код" }
                                td { class: "py-3 pr-4", CheckBadge { label: "Лицензия MIT" } }
                                td { class: "py-3 text-[13px] text-zinc-600", "Закрытый код" }
                            }
                            tr {
                                td { class: "py-3 pr-4 text-[13px] text-zinc-400", "Самостоятельный хостинг" }
                                td { class: "py-3 pr-4", CheckBadge { label: "Docker Compose" } }
                                td { class: "py-3 text-[13px] text-zinc-600", "Нет" }
                            }
                            tr {
                                td { class: "py-3 pr-4 text-[13px] text-zinc-400", "Трекинг и аналитика" }
                                td { class: "py-3 pr-4", CheckBadge { label: "Нет" } }
                                td { class: "py-3 text-[13px] text-zinc-600", "Да, обширная аналитика" }
                            }
                            tr {
                                td { class: "py-3 pr-4 text-[13px] text-zinc-400", "Потребление RAM" }
                                td { class: "py-3 pr-4 text-[13px] text-zinc-200", "< 120 MB в браузере" }
                                td { class: "py-3 text-[13px] text-zinc-600", "400 MB–1 GB+ в Electron" }
                            }
                            tr {
                                td { class: "py-3 pr-4 text-[13px] text-zinc-400", "Публичная разработка" }
                                td { class: "py-3 pr-4", CheckBadge { label: "Стримы на YouTube" } }
                                td { class: "py-3 text-[13px] text-zinc-600", "Нет" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TechSection() -> Element {
    rsx! {
        section { id: "tech", class: "mx-auto max-w-6xl px-5 pb-20 lg:px-8",
            SectionIntro {
                eyebrow: "Стек",
                title: "Технический стек.",
                description: "Без enterprise-раздутости и legacy-подходов. Весь код — на GitHub."
            }
            div { class: "grid gap-3 sm:grid-cols-2 lg:grid-cols-4",
                for group in TECH_GROUPS {
                    TechCard { group: *group }
                }
            }
            div { class: "mt-4 rounded-[20px] border border-zinc-800 bg-zinc-950/80 p-5",
                div { class: "flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between",
                    div {
                        div { class: "font-mono text-[11px] text-zinc-500", "$ git clone https://github.com/loject/cheenhub" }
                        div { class: "mt-1.5 text-[13px] text-zinc-400", "Весь исходный код открыт. Issues, PR и обсуждения — добро пожаловать." }
                    }
                    a {
                        href: "https://github.com/loject/cheenhub",
                        target: "_blank",
                        rel: "noopener",
                        class: "btn-g flex shrink-0 items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2.5 text-[13px] font-medium text-zinc-200",
                        GithubIcon { class_name: "h-4 w-4" }
                        "Открыть репозиторий"
                    }
                }
            }
        }
    }
}

#[component]
fn CtaSection() -> Element {
    rsx! {
        section { class: "mx-auto max-w-6xl px-5 pb-24 lg:px-8",
            div { class: "relative overflow-visible rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-10 text-center shadow-[0_0_0_1px_rgba(255,255,255,0.02)]",
                div { class: "pointer-events-none absolute left-1/2 top-0 h-72 w-[500px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-accent/7 blur-3xl" }
                div { class: "relative",
                    div { class: "mb-3 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-950 px-3 py-1 text-[10px] uppercase tracking-[0.24em] text-zinc-500",
                        span { class: "h-1.5 w-1.5 rounded-full bg-accent/70" }
                        "Следи за разработкой"
                    }
                    h2 { class: "text-2xl font-semibold tracking-[-0.04em] text-zinc-50 sm:text-3xl", "Разработка в прямом эфире." }
                    p { class: "mx-auto mt-3 max-w-lg text-[14px] leading-relaxed text-zinc-500",
                        "Каждый стрим — реальный коммит. Смотри как строится продукт,"
                        br { class: "hidden sm:inline" }
                        "задавай вопросы, предлагай фичи прямо в чате."
                    }
                    div { class: "mt-7 flex flex-wrap items-center justify-center gap-3",
                        WebButton { large: true }
                        DownloadDropdown { opens_up: true, large: true }
                    }
                    SocialLinks { class_name: "mt-5 flex flex-wrap items-center justify-center gap-4 text-[13px] text-zinc-500", hover_class: "transition hover:text-zinc-200" }
                }
            }
        }
    }
}

#[component]
fn Footer() -> Element {
    rsx! {
        footer { class: "border-t border-zinc-800/80 bg-zinc-950/80",
            div { class: "mx-auto flex max-w-6xl flex-col items-center justify-between gap-3 px-5 py-6 text-[12px] text-zinc-600 sm:flex-row lg:px-8",
                div { class: "flex items-center gap-2",
                    div { class: "flex h-6 w-6 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900 text-zinc-500",
                        LogoIcon { class_name: "h-3.5 w-3.5" }
                    }
                    "CheenHub — лицензия MIT"
                }
                div { class: "flex flex-wrap items-center justify-center gap-4 sm:justify-end",
                    a { href: "https://github.com/loject/cheenhub", target: "_blank", rel: "noopener", class: "transition hover:text-zinc-400", "GitHub" }
                    a { href: "https://youtube.com/@cheenhub", target: "_blank", rel: "noopener", class: "transition hover:text-zinc-400", "YouTube" }
                    TodoButton { class_name: "bg-transparent p-0 transition hover:text-zinc-400", label: "RuTube" }
                    TodoButton { class_name: "bg-transparent p-0 transition hover:text-zinc-400", label: "Telegram" }
                    span { class: "text-zinc-800", "·" }
                    span { "Разработка публично" }
                }
            }
        }
    }
}

#[component]
fn SectionIntro(eyebrow: &'static str, title: &'static str, description: &'static str) -> Element {
    rsx! {
        div { class: "mb-7",
            Eyebrow { label: eyebrow, dark: false }
            h2 { class: "text-2xl font-semibold tracking-[-0.04em] text-zinc-50", "{title}" }
            p { class: "mt-2 text-[14px] text-zinc-500", "{description}" }
        }
    }
}

#[component]
fn Eyebrow(label: &'static str, dark: bool) -> Element {
    let class_name = if dark {
        "mb-2 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-950 px-3 py-1 text-[10px] uppercase tracking-[0.24em] text-zinc-500"
    } else {
        "mb-2 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/80 px-3 py-1 text-[10px] uppercase tracking-[0.24em] text-zinc-500"
    };

    rsx! {
        div { class: "{class_name}",
            span { class: "h-1.5 w-1.5 rounded-full bg-zinc-600" }
            "{label}"
        }
    }
}

#[component]
fn FeatureCard(feature: Feature) -> Element {
    rsx! {
        div { class: "fcard rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-5",
            div { class: "mb-3 flex h-9 w-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-300",
                FeatureSvg { icon: feature.icon }
            }
            div { class: "text-[13px] font-semibold text-zinc-100", "{feature.title}" }
            div { class: "mt-1.5 text-[13px] leading-5 text-zinc-500", "{feature.description}" }
        }
    }
}

#[component]
fn TechCard(group: TechGroup) -> Element {
    rsx! {
        div { class: "rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-4",
            div { class: "mb-3 text-[10px] uppercase tracking-[0.2em] text-zinc-600", "{group.title}" }
            div { class: "space-y-2",
                for item in group.items {
                    div { class: "flex items-center gap-2.5 rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2",
                        span { class: "w-6 text-center font-mono text-[12px] text-zinc-400", "{item.code}" }
                        span { class: "text-[12px] text-zinc-300", "{item.label}" }
                    }
                }
            }
        }
    }
}

#[component]
fn SocialLinks(class_name: &'static str, hover_class: &'static str) -> Element {
    rsx! {
        div { class: "{class_name}",
            a { href: "https://github.com/loject/cheenhub", target: "_blank", rel: "noopener", class: "{hover_class}", "GitHub" }
            a { href: "https://youtube.com/@cheenhub", target: "_blank", rel: "noopener", class: "{hover_class}", "YouTube" }
            TodoButton { class_name: hover_class, label: "RuTube" }
            TodoButton { class_name: hover_class, label: "Telegram" }
        }
    }
}

#[component]
fn Pill(strong: &'static str, text: &'static str) -> Element {
    rsx! {
        div { class: "flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/80 px-3.5 py-1.5 text-[11px] text-zinc-400",
            span { class: "font-medium text-zinc-200", "{strong}" }
            "{text}"
        }
    }
}

#[component]
fn WebButton(large: bool) -> Element {
    let class_name = if large {
        "btn-p flex items-center gap-2 rounded-xl bg-accent px-6 py-3 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)]"
    } else {
        "btn-p flex items-center gap-2 rounded-xl bg-accent px-5 py-2.5 text-[13px] font-medium text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_4px_18px_rgba(59,130,246,0.16)]"
    };

    rsx! {
        a {
            href: "https://cheenhub.ru/web",
            target: "_blank",
            rel: "noopener",
            class: "{class_name}",
            GlobeIcon { class_name: "h-4 w-4" }
            "Открыть веб-версию"
        }
    }
}

#[component]
fn DownloadDropdown(opens_up: bool, large: bool) -> Element {
    let mut is_open = use_signal(|| false);
    let button_class = if large {
        "btn-g flex items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900 px-6 py-3 text-[13px] font-semibold text-zinc-200"
    } else {
        "btn-g flex items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900 px-5 py-2.5 text-[13px] font-medium text-zinc-200"
    };
    let menu_class = match (opens_up, is_open()) {
        (true, true) => {
            "absolute left-0 bottom-full z-[80] mb-2 min-w-[220px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950/95 p-1.5 text-left shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl"
        }
        (true, false) => {
            "absolute left-0 bottom-full z-[80] mb-2 hidden min-w-[220px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950/95 p-1.5 text-left shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl"
        }
        (false, true) => {
            "absolute left-0 top-full z-[80] mt-2 min-w-[220px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950/95 p-1.5 shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl"
        }
        (false, false) => {
            "absolute left-0 top-full z-[80] mt-2 hidden min-w-[220px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950/95 p-1.5 shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl"
        }
    };
    let expanded = if is_open() { "true" } else { "false" };

    rsx! {
        div { class: "relative z-[70]",
            button {
                r#type: "button",
                aria_expanded: "{expanded}",
                class: "{button_class}",
                onclick: move |_| is_open.set(!is_open()),
                DownloadIcon { class_name: "h-4 w-4" }
                "Скачать нативный клиент"
                ChevronDownIcon { class_name: "h-3.5 w-3.5 text-zinc-500" }
            }
            div { class: "{menu_class}",
                DownloadLink { href: "https://cheenhub.ru/download/windows", label: "Windows", format: ".msi" }
                DownloadLink { href: "https://cheenhub.ru/download/linux", label: "Ubuntu/Linux", format: ".deb" }
                DownloadLink { href: "https://cheenhub.ru/download/android", label: "Android", format: ".apk" }
            }
        }
    }
}

#[component]
fn DownloadLink(href: &'static str, label: &'static str, format: &'static str) -> Element {
    rsx! {
        a {
            href,
            class: "flex items-center justify-between rounded-xl px-3 py-2 text-[13px] text-zinc-300 transition hover:bg-zinc-900 hover:text-zinc-100",
            span { "{label}" }
            span { class: "text-[11px] text-zinc-600", "{format}" }
        }
    }
}

#[component]
fn TodoButton(class_name: &'static str, label: &'static str) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "{class_name}",
            onclick: move |_| show_todo_alert(),
            "{label}"
        }
    }
}

#[component]
fn CheckBadge(label: &'static str) -> Element {
    rsx! {
        span { class: "inline-flex items-center gap-1.5 rounded-full border border-zinc-700 bg-zinc-900 px-2.5 py-1 text-[11px] text-zinc-300",
            CheckIcon { class_name: "h-3 w-3" }
            "{label}"
        }
    }
}

#[component]
fn LogoIcon(class_name: &'static str) -> Element {
    rsx! {
        svg { class: "{class_name}", view_box: "0 0 1024 1024",
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

#[component]
fn FeatureSvg(icon: FeatureIcon) -> Element {
    match icon {
        FeatureIcon::Phone => rsx! {
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 5a2 2 0 0 1 2-2h3.28a1 1 0 0 1 .948.684l1.498 4.493a1 1 0 0 1-.502 1.21l-2.257 1.13a11.042 11.042 0 0 0 5.516 5.516l1.13-2.257a1 1 0 0 1 1.21-.502l4.493 1.498a1 1 0 0 1 .684.949V19a2 2 0 0 1-2 2h-1C9.716 21 3 14.284 3 6V5Z" }
            }
        },
        FeatureIcon::Users => rsx! {
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M17 20h5v-2a3 3 0 0 0-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 0 1 5.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 0 1 9.288 0M15 7a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" }
            }
        },
        FeatureIcon::Screen => rsx! {
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24",
                rect { x: "2", y: "3", width: "20", height: "14", rx: "2" }
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 21h8m-4-4v4" }
            }
        },
        FeatureIcon::Code => rsx! {
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10 20l4-16m4 4 4 4-4 4M6 16l-4-4 4-4" }
            }
        },
        FeatureIcon::Shield => rsx! {
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10Z" }
            }
        },
        FeatureIcon::CheckShield => rsx! {
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "m9 12 2 2 4-4m5.618-4.016A11.955 11.955 0 0 1 12 2.944a11.955 11.955 0 0 1-8.618 3.04A12.02 12.02 0 0 0 3 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016Z" }
            }
        },
    }
}

#[component]
fn GlobeIcon(class_name: &'static str) -> Element {
    rsx! {
        svg { class: "{class_name}", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 12h18M12 3c2.5 2.5 4 5.75 4 9s-1.5 6.5-4 9m0-18c-2.5 2.5-4 5.75-4 9s1.5 6.5 4 9" }
        }
    }
}

#[component]
fn DownloadIcon(class_name: &'static str) -> Element {
    rsx! {
        svg { class: "{class_name}", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 16V4m0 12 4-4m-4 4-4-4M5 20h14" }
        }
    }
}

#[component]
fn ChevronDownIcon(class_name: &'static str) -> Element {
    rsx! {
        svg { class: "{class_name}", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "m6 9 6 6 6-6" }
        }
    }
}

#[component]
fn CheckIcon(class_name: &'static str) -> Element {
    rsx! {
        svg { class: "{class_name}", fill: "none", stroke: "currentColor", stroke_width: "2.2", view_box: "0 0 24 24",
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 13l4 4L19 7" }
        }
    }
}

#[component]
fn GithubIcon(class_name: &'static str) -> Element {
    rsx! {
        svg { class: "{class_name}", fill: "currentColor", view_box: "0 0 24 24",
            path { d: "M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0 1 12 6.844a9.59 9.59 0 0 1 2.504.337c1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.02 10.02 0 0 0 22 12.017C22 6.484 17.522 2 12 2Z" }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn show_todo_alert() {
    if let Some(window) = web_sys::window() {
        let _ = window.alert_with_message("TODO");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn show_todo_alert() {}
