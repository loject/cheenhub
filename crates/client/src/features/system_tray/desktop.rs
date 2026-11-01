//! Desktop-реализация системного трея.

use dioxus::desktop::{
    WindowCloseBehaviour, icon_from_memory, trayicon, use_muda_event_handler, use_window,
};
use dioxus::prelude::*;

use super::super::status::VoiceIconStatus;

const OPEN_MENU_ID: &str = "cheenhub.system_tray.open";
const QUIT_MENU_ID: &str = "cheenhub.system_tray.quit";

/// Сообщает, поддерживается ли системный трей на текущей платформе.
pub(crate) const fn is_supported() -> bool {
    true
}

/// Подключает desktop-трей и синхронизирует поведение закрытия окна.
#[component]
pub(crate) fn SystemTrayPlatformEffects(
    minimize_to_tray_on_close: Signal<bool>,
    voice_status: Signal<VoiceIconStatus>,
) -> Element {
    let window = use_window();
    let tray_window = window.clone();
    let _tray_icon = use_hook(|| {
        let tray_icon = trayicon::init_tray_icon(
            build_tray_menu(),
            icon_from_memory(include_bytes!(
                "../../../../../crates/client/public/icons/icon-512.png"
            ))
            .ok(),
        );
        if let Err(error) = tray_icon.set_tooltip(Some("CheenHub")) {
            warn!(%error, "failed to set system tray tooltip");
        }
        info!("initialized CheenHub system tray icon");
        tray_icon
    });

    let activity_tray_icon = _tray_icon.clone();
    use_effect(move || {
        let status = voice_status();
        if let Err(error) = activity_tray_icon.set_icon(status_icon(status)) {
            warn!(?status, %error, "failed to update system tray voice indicator");
        } else {
            debug!(?status, "updated system tray voice indicator");
        }
    });

    use_effect(move || {
        let enabled = minimize_to_tray_on_close();
        let behaviour = if enabled {
            WindowCloseBehaviour::WindowHides
        } else {
            WindowCloseBehaviour::WindowCloses
        };
        window.set_close_behavior(behaviour);
        info!(
            enabled,
            "updated desktop close behavior from system tray preference"
        );
    });

    // Dioxus/muda передаёт события меню трея через общий обработчик меню.
    use_muda_event_handler(move |event| match event.id().as_ref() {
        OPEN_MENU_ID => {
            tray_window.set_visible(true);
            tray_window.set_focus();
            info!("opened CheenHub window from system tray menu");
        }
        QUIT_MENU_ID => {
            quit_from_system_tray();
        }
        _ => {}
    });

    rsx! {}
}

/// Создаёт значок приложения с одним голосовым статусом.
fn status_icon(status: VoiceIconStatus) -> Option<trayicon::DioxusTrayIcon> {
    use image::ImageEncoder;

    if status == VoiceIconStatus::Inactive {
        return default_voice_icon();
    }

    let mut image = image::load_from_memory(include_bytes!(
        "../../../../../crates/client/public/icons/icon-512.png"
    ))
    .ok()?
    .to_rgba8();
    let size = image.width().min(image.height());
    let center_x = image.width() * 3 / 4;
    let center_y = image.height() * 3 / 4;
    let radius = size / 4;
    match status {
        VoiceIconStatus::Inactive => {}
        VoiceIconStatus::Connected => {
            draw_status_badge(&mut image, center_x, center_y, radius, [34, 197, 94, 255]);
        }
        VoiceIconStatus::Speaking => {
            draw_status_badge(&mut image, center_x, center_y, radius, [134, 239, 172, 255]);
        }
        VoiceIconStatus::MicrophoneMuted => {
            draw_muted_symbol(&mut image, center_x, center_y, radius, false);
        }
        VoiceIconStatus::OutputMuted => {
            draw_muted_symbol(&mut image, center_x, center_y, radius, true);
        }
        VoiceIconStatus::BothMuted => {
            draw_muted_symbol(&mut image, center_x / 3, center_y, radius * 3 / 4, false);
            draw_muted_symbol(&mut image, center_x, center_y, radius * 3 / 4, true);
        }
    }

    let mut encoded = Vec::new();
    image::codecs::png::PngEncoder::new(&mut encoded)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .ok()?;
    icon_from_memory(&encoded).ok().or_else(default_voice_icon)
}

/// Создаёт обычный значок приложения для системного трея.
fn default_voice_icon() -> Option<trayicon::DioxusTrayIcon> {
    icon_from_memory(include_bytes!(
        "../../../../../crates/client/public/icons/icon-512.png"
    ))
    .ok()
}

/// Рисует крупный круглый маркер с контрастной тёмной окантовкой.
fn draw_status_badge(
    image: &mut image::RgbaImage,
    center_x: u32,
    center_y: u32,
    radius: u32,
    color: [u8; 4],
) {
    let width = image.width();
    let height = image.height();
    let outer_radius = radius + radius / 7;
    let outer_squared = i64::from(outer_radius) * i64::from(outer_radius);
    let inner_squared = i64::from(radius) * i64::from(radius);
    let left = center_x.saturating_sub(outer_radius);
    let right = center_x
        .saturating_add(outer_radius)
        .min(width.saturating_sub(1));
    let top = center_y.saturating_sub(outer_radius);
    let bottom = center_y
        .saturating_add(outer_radius)
        .min(height.saturating_sub(1));

    for y in top..=bottom {
        for x in left..=right {
            let dx = i64::from(x) - i64::from(center_x);
            let dy = i64::from(y) - i64::from(center_y);
            let distance_squared = dx * dx + dy * dy;
            if distance_squared <= outer_squared {
                let pixel = if distance_squared <= inner_squared {
                    image::Rgba(color)
                } else {
                    image::Rgba([9, 9, 11, 255])
                };
                image.put_pixel(x, y, pixel);
            }
        }
    }
}

/// Рисует контрастный перечёркнутый микрофон или наушники.
fn draw_muted_symbol(
    image: &mut image::RgbaImage,
    cx: u32,
    cy: u32,
    radius: u32,
    headphones: bool,
) {
    draw_status_badge(image, cx, cy, radius, [24, 24, 27, 255]);
    let r = radius as i32;
    let (cx, cy) = (cx as i32, cy as i32);
    let white = image::Rgba([255, 255, 255, 255]);
    let red = image::Rgba([248, 113, 113, 255]);
    let stroke = (r / 8).max(3);
    if headphones {
        // Дуга наушников и две чашки.
        for step in 0..=100 {
            let angle = std::f32::consts::PI + std::f32::consts::PI * step as f32 / 100.0;
            // Координаты изображения направлены вниз: верхняя дуга соответствует sin <= 0.
            let x = cx + (r as f32 * 0.55 * angle.cos()) as i32;
            let y = cy + (r as f32 * 0.55 * angle.sin()) as i32;
            paint_disk(image, x, y, stroke, white);
        }
        for dx in [-1, 1] {
            let x = cx + dx * r * 55 / 100;
            for y in cy - r / 6..=cy + r / 2 {
                paint_disk(image, x, y, stroke, white);
            }
        }
    } else {
        // Капсюль микрофона, ножка и основание.
        for y in cy - r / 2..=cy + r / 5 {
            paint_disk(image, cx, y, r / 5, white);
        }
        for y in cy + r / 5..=cy + r / 2 {
            paint_disk(image, cx, y, stroke, white);
        }
        for x in cx - r / 3..=cx + r / 3 {
            paint_disk(image, x, cy + r / 2, stroke, white);
        }
    }
    for delta in -r * 2 / 3..=r * 2 / 3 {
        paint_disk(image, cx + delta, cy - delta, stroke + 2, red);
    }
}

fn paint_disk(image: &mut image::RgbaImage, cx: i32, cy: i32, radius: i32, color: image::Rgba<u8>) {
    for y in cy - radius..=cy + radius {
        for x in cx - radius..=cx + radius {
            if x >= 0
                && y >= 0
                && x < image.width() as i32
                && y < image.height() as i32
                && (x - cx).pow(2) + (y - cy).pow(2) <= radius.pow(2)
            {
                image.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}

fn build_tray_menu() -> trayicon::DioxusTrayMenu {
    use trayicon::menu::{Menu, MenuItem, PredefinedMenuItem};

    let open = MenuItem::with_id(OPEN_MENU_ID, "Открыть CheenHub", true, None);
    let separator = PredefinedMenuItem::separator();
    let quit = MenuItem::with_id(QUIT_MENU_ID, "Выйти", true, None);
    let menu = Menu::new();
    menu.append_items(&[&open, &separator, &quit])
        .expect("failed to build CheenHub tray menu");
    menu
}

fn quit_from_system_tray() -> ! {
    info!(exit_code = 0, "exiting CheenHub from system tray menu");
    std::process::exit(0);
}
