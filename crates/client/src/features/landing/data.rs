//! Static data for the landing page.

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Feature {
    pub(crate) title: &'static str,
    pub(crate) description: &'static str,
    pub(crate) icon: FeatureIcon,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum FeatureIcon {
    Phone,
    Users,
    Screen,
    Code,
    Shield,
    CheckShield,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct TechItem {
    pub(crate) code: &'static str,
    pub(crate) label: &'static str,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct TechGroup {
    pub(crate) title: &'static str,
    pub(crate) items: &'static [TechItem],
}

pub(crate) const FEATURES: &[Feature] = &[
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

pub(crate) const TECH_GROUPS: &[TechGroup] = &[
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
