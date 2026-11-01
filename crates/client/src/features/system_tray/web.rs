//! Браузерный индикатор голосового состояния во favicon.

use dioxus::prelude::*;

use super::super::status::VoiceIconStatus;

/// В браузере системный трей не поддерживается.
pub(crate) const fn is_supported() -> bool {
    false
}

/// Синхронизирует favicon с состоянием голосового подключения.
#[component]
pub(crate) fn SystemTrayPlatformEffects(
    minimize_to_tray_on_close: Signal<bool>,
    voice_status: Signal<VoiceIconStatus>,
) -> Element {
    let _ = minimize_to_tray_on_close;
    use_effect(move || {
        let status = voice_status();
        let script = format!(
            r##"(() => {{
                const existing = document.querySelector('link[rel="icon"]');
                if (!existing) return;
                if (!existing.dataset.cheenhubOriginalIcon) {{
                    existing.dataset.cheenhubOriginalIcon = existing.getAttribute('href') || '/icons/favicon.svg';
                }}
                const original = existing.dataset.cheenhubOriginalIcon;
                if ({inactive}) {{
                    existing.setAttribute('href', original);
                    return;
                }}
                const symbol = {symbol};
                const logo = {logo};
                const logoContent = logo.slice(logo.indexOf('>') + 1, logo.lastIndexOf('</svg>'));
                const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><g transform="scale(0.1175) translate(-503.703636 -225.558456)">${{logoContent}}</g><circle cx="47" cy="47" r="17" fill="#18181b" stroke="#ffffff" stroke-width="2"/>${{symbol}}</svg>`;
                existing.setAttribute('href', 'data:image/svg+xml,' + encodeURIComponent(svg));
            }})();"##,
            inactive = status == VoiceIconStatus::Inactive,
            symbol = js_symbol(status),
            logo = serde_json::to_string(include_str!("../../../public/icons/favicon.svg"))
                .unwrap_or_default(),
        );
        document::eval(&script);
    });
    rsx! {}
}

fn js_symbol(status: VoiceIconStatus) -> &'static str {
    match status {
        VoiceIconStatus::Inactive => "''",
        VoiceIconStatus::Connected => "'<circle cx=\"47\" cy=\"47\" r=\"12\" fill=\"#22c55e\"/>'",
        VoiceIconStatus::Speaking => "'<circle cx=\"47\" cy=\"47\" r=\"12\" fill=\"#86efac\"/>'",
        VoiceIconStatus::MicrophoneMuted => {
            "'<path d=\"M47 38v10m-4-10v10m8-10v10M41 51h12m-6 0v4\" stroke=\"white\" stroke-width=\"3\" stroke-linecap=\"round\"/><path d=\"M37 57l20-20\" stroke=\"#f87171\" stroke-width=\"4\"/>'"
        }
        VoiceIconStatus::OutputMuted => {
            "'<path d=\"M38 48v-5a9 9 0 0 1 18 0v5m-18-2v7m18-7v7\" fill=\"none\" stroke=\"white\" stroke-width=\"4\" stroke-linecap=\"round\"/><path d=\"M37 57l20-20\" stroke=\"#f87171\" stroke-width=\"4\"/>'"
        }
        VoiceIconStatus::BothMuted => {
            "'<path d=\"M37 42v9m-3-9v9m6-9v9m-6 2h7m-4 0v3M48 49v-7a6 6 0 0 1 12 0v7m-12-1v6m12-6v6\" stroke=\"white\" fill=\"none\" stroke-width=\"2.5\"/><path d=\"M33 57l29-22\" stroke=\"#f87171\" stroke-width=\"4\"/>'"
        }
    }
}
