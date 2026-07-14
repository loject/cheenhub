//! Shared user settings style helpers.

/// Base field class for compact text inputs.
pub(crate) fn input_class() -> &'static str {
    "h-10 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] text-zinc-100 outline-none transition placeholder:text-zinc-700 focus:border-accent/70 focus:ring-4 focus:ring-accent/10"
}

/// Base select class for compact settings controls.
pub(crate) fn select_class() -> &'static str {
    "h-10 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] text-zinc-100 outline-none transition focus:border-accent/70 focus:ring-4 focus:ring-accent/10"
}

/// Primary action button class.
pub(crate) fn primary_button_class() -> &'static str {
    "flex h-10 w-full items-center justify-center rounded-xl bg-accent px-4 text-[12px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.16)] transition hover:bg-blue-400 disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-accent sm:h-9 sm:w-auto"
}

/// Parses a percentage-like value and clamps it to a slider range.
pub(crate) fn parse_percent(value: &str, fallback: i32) -> i32 {
    value.parse::<i32>().unwrap_or(fallback).clamp(0, 100)
}

/// Parses a percentage-like value and clamps it to a custom slider range.
pub(crate) fn parse_percent_range(value: &str, fallback: u32, max: u32) -> u32 {
    value.parse::<u32>().unwrap_or(fallback).min(max)
}
