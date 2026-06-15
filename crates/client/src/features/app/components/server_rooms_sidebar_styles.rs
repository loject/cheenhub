//! Style helpers for the server rooms sidebar.

pub(super) fn rooms_sidebar_class(settings_workspace_active: bool) -> &'static str {
    if settings_workspace_active {
        "server-rooms-sidebar group/rooms relative z-30 flex w-[284px] shrink-0 flex-col border-r border-zinc-800/80 bg-zinc-950/85 backdrop-blur-xl transition-[width] duration-200 ease-out max-[1440px]:w-[68px] max-[1440px]:hover:w-[284px] max-[1440px]:focus-within:w-[284px]"
    } else {
        "server-rooms-sidebar group/rooms relative z-30 flex w-[284px] shrink-0 flex-col border-r border-zinc-800/80 bg-zinc-950/85 backdrop-blur-xl transition-[width] duration-200 ease-out"
    }
}

pub(super) fn rooms_sidebar_header_text_class(settings_workspace_active: bool) -> &'static str {
    if settings_workspace_active {
        "min-w-0 flex-1 overflow-hidden transition-[opacity] duration-150 max-[1440px]:opacity-0 max-[1440px]:group-hover/rooms:opacity-100 max-[1440px]:group-focus-within/rooms:opacity-100"
    } else {
        "min-w-0 flex-1 overflow-hidden transition-[opacity] duration-150"
    }
}

pub(super) fn rooms_sidebar_header_icon_class(settings_workspace_active: bool) -> &'static str {
    if settings_workspace_active {
        "h-4 w-4 shrink-0 text-zinc-500 transition-[opacity] duration-150 max-[1440px]:opacity-0 max-[1440px]:group-hover/rooms:opacity-100 max-[1440px]:group-focus-within/rooms:opacity-100"
    } else {
        "h-4 w-4 shrink-0 text-zinc-500 transition-[opacity] duration-150"
    }
}

pub(super) fn room_section_title_class(settings_workspace_active: bool) -> &'static str {
    if settings_workspace_active {
        "overflow-hidden whitespace-nowrap transition-[opacity] duration-150 max-[1440px]:opacity-0 max-[1440px]:group-hover/rooms:opacity-100 max-[1440px]:group-focus-within/rooms:opacity-100"
    } else {
        "overflow-hidden whitespace-nowrap transition-[opacity] duration-150"
    }
}

pub(super) fn connection_status_class(settings_workspace_active: bool) -> &'static str {
    if settings_workspace_active {
        "mb-2 flex items-center gap-2 overflow-visible rounded-[16px] border border-zinc-800 bg-zinc-900/70 px-3 py-2 max-[1440px]:justify-center max-[1440px]:group-hover/rooms:justify-start max-[1440px]:group-focus-within/rooms:justify-start"
    } else {
        "mb-2 flex items-center gap-2 overflow-visible rounded-[16px] border border-zinc-800 bg-zinc-900/70 px-3 py-2"
    }
}

pub(super) fn connection_details_class(settings_workspace_active: bool) -> &'static str {
    if settings_workspace_active {
        "min-w-0 flex-1 transition-[opacity] duration-150 max-[1440px]:hidden max-[1440px]:opacity-0 max-[1440px]:group-hover/rooms:block max-[1440px]:group-hover/rooms:opacity-100 max-[1440px]:group-focus-within/rooms:block max-[1440px]:group-focus-within/rooms:opacity-100"
    } else {
        "min-w-0 flex-1 transition-[opacity] duration-150"
    }
}

pub(super) fn sidebar_voice_class(settings_workspace_active: bool) -> &'static str {
    if settings_workspace_active {
        "overflow-hidden transition-[opacity] duration-150 max-[1440px]:hidden max-[1440px]:opacity-0 max-[1440px]:group-hover/rooms:block max-[1440px]:group-hover/rooms:opacity-100 max-[1440px]:group-focus-within/rooms:block max-[1440px]:group-focus-within/rooms:opacity-100"
    } else {
        "overflow-hidden transition-[opacity] duration-150"
    }
}

pub(super) fn user_bar_class(settings_workspace_active: bool) -> &'static str {
    if settings_workspace_active {
        "flex items-center gap-3 overflow-hidden rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-2.5 max-[1440px]:justify-center max-[1440px]:group-hover/rooms:justify-start max-[1440px]:group-focus-within/rooms:justify-start"
    } else {
        "flex items-center gap-3 overflow-hidden rounded-[20px] border border-zinc-800 bg-zinc-900/80 p-2.5"
    }
}

pub(super) fn user_details_class(settings_workspace_active: bool) -> &'static str {
    if settings_workspace_active {
        "min-w-0 flex-1 transition-[opacity] duration-150 max-[1440px]:hidden max-[1440px]:opacity-0 max-[1440px]:group-hover/rooms:block max-[1440px]:group-hover/rooms:opacity-100 max-[1440px]:group-focus-within/rooms:block max-[1440px]:group-focus-within/rooms:opacity-100"
    } else {
        "min-w-0 flex-1 transition-[opacity] duration-150"
    }
}
