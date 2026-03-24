use tray_icon::menu::{Menu, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};

pub struct TrayState {
    pub capture_id: tray_icon::menu::MenuId,
    pub about_id: tray_icon::menu::MenuId,
    pub quit_id: tray_icon::menu::MenuId,
    pub _tray: tray_icon::TrayIcon,
}

pub fn create_tray() -> Result<TrayState, String> {
    // Load icon from embedded bytes
    let icon_bytes = include_bytes!("../assets/icon.png");
    let icon_img = image::load_from_memory(icon_bytes)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let (w, h) = icon_img.dimensions();
    let icon = Icon::from_rgba(icon_img.into_raw(), w, h).map_err(|e| e.to_string())?;

    // Create menu with Capture, About, Quit
    let menu = Menu::new();
    let capture_item = MenuItem::new("Capture", true, None);
    let about_item = MenuItem::new("About", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let capture_id = capture_item.id().clone();
    let about_id = about_item.id().clone();
    let quit_id = quit_item.id().clone();
    menu.append(&capture_item).map_err(|e| e.to_string())?;
    menu.append(&about_item).map_err(|e| e.to_string())?;
    menu.append(&quit_item).map_err(|e| e.to_string())?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("HydroShot")
        .with_icon(icon)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(TrayState {
        capture_id,
        about_id,
        quit_id,
        _tray: tray,
    })
}
