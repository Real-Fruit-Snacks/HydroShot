use tray_icon::menu::{CheckMenuItem, Menu, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};

pub struct TrayState {
    pub capture_id: tray_icon::menu::MenuId,
    pub window_capture_id: tray_icon::menu::MenuId,
    pub delay_3_id: tray_icon::menu::MenuId,
    pub delay_5_id: tray_icon::menu::MenuId,
    pub delay_10_id: tray_icon::menu::MenuId,
    pub history_id: tray_icon::menu::MenuId,
    pub autostart_id: tray_icon::menu::MenuId,
    pub settings_id: tray_icon::menu::MenuId,
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
    let window_capture_item = MenuItem::new("Capture Window", true, None);
    let delay_3_item = MenuItem::new("Capture in 3s", true, None);
    let delay_5_item = MenuItem::new("Capture in 5s", true, None);
    let delay_10_item = MenuItem::new("Capture in 10s", true, None);
    let history_item = MenuItem::new("History", true, None);
    let autostart_item =
        CheckMenuItem::new("Start on login", true, crate::autostart::is_enabled(), None);
    let settings_item = MenuItem::new("Settings", true, None);
    let about_item = MenuItem::new("About", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let capture_id = capture_item.id().clone();
    let window_capture_id = window_capture_item.id().clone();
    let delay_3_id = delay_3_item.id().clone();
    let delay_5_id = delay_5_item.id().clone();
    let delay_10_id = delay_10_item.id().clone();
    let history_id = history_item.id().clone();
    let autostart_id = autostart_item.id().clone();
    let settings_id = settings_item.id().clone();
    let about_id = about_item.id().clone();
    let quit_id = quit_item.id().clone();
    menu.append(&capture_item).map_err(|e| e.to_string())?;
    menu.append(&window_capture_item)
        .map_err(|e| e.to_string())?;
    menu.append(&delay_3_item).map_err(|e| e.to_string())?;
    menu.append(&delay_5_item).map_err(|e| e.to_string())?;
    menu.append(&delay_10_item).map_err(|e| e.to_string())?;
    menu.append(&history_item).map_err(|e| e.to_string())?;
    menu.append(&autostart_item).map_err(|e| e.to_string())?;
    menu.append(&settings_item).map_err(|e| e.to_string())?;
    menu.append(&about_item).map_err(|e| e.to_string())?;
    menu.append(&quit_item).map_err(|e| e.to_string())?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip("HydroShot")
        .with_icon(icon)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(TrayState {
        capture_id,
        window_capture_id,
        delay_3_id,
        delay_5_id,
        delay_10_id,
        history_id,
        autostart_id,
        settings_id,
        about_id,
        quit_id,
        _tray: tray,
    })
}
