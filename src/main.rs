use std::sync::Arc;

use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

/// Application state for the HydroShot spike.
struct App {
    /// The currently open capture window, if any.
    window: Option<Arc<Window>>,
    /// Menu item IDs for matching events.
    capture_menu_id: String,
    quit_menu_id: String,
}

impl App {
    fn new(capture_menu_id: String, quit_menu_id: String) -> Self {
        Self {
            window: None,
            capture_menu_id,
            quit_menu_id,
        }
    }

    /// Open a borderless 800x600 test window.
    fn open_capture_window(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            tracing::info!("Capture window already open");
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("HydroShot Capture")
            .with_inner_size(LogicalSize::new(800.0, 600.0))
            .with_decorations(false);

        match event_loop.create_window(attrs) {
            Ok(win) => {
                tracing::info!("Capture window opened");
                self.window = Some(Arc::new(win));
            }
            Err(e) => {
                tracing::error!("Failed to create window: {e}");
            }
        }
    }

    /// Close the capture window.
    fn close_capture_window(&mut self) {
        if self.window.take().is_some() {
            tracing::info!("Capture window closed");
        }
    }

    /// Process pending tray icon and menu events.
    fn process_tray_events(&mut self, event_loop: &ActiveEventLoop) {
        // Handle tray icon click events.
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    tracing::info!("Capture triggered!");
                    self.open_capture_window(event_loop);
                }
                _ => {}
            }
        }

        // Handle menu events.
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id.0 == self.capture_menu_id {
                tracing::info!("Capture menu item clicked");
                self.open_capture_window(event_loop);
            } else if event.id.0 == self.quit_menu_id {
                tracing::info!("Quit requested");
                event_loop.exit();
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        tracing::info!("Application resumed");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.close_capture_window();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::keyboard::{Key, NamedKey};
                if event.logical_key == Key::Named(NamedKey::Escape) && event.state.is_pressed() {
                    tracing::info!("Escape pressed, closing window");
                    self.close_capture_window();
                }
            }
            _ => {}
        }
        // If no windows remain, keep running (tray-only mode).
        let _ = event_loop;
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_tray_events(event_loop);
    }
}

fn load_tray_icon() -> Icon {
    let icon_bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(icon_bytes)
        .expect("Failed to load tray icon")
        .into_rgba8();
    let (w, h) = img.dimensions();
    Icon::from_rgba(img.into_raw(), w, h).expect("Failed to create icon from RGBA data")
}

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("HydroShot starting");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    // Build tray menu.
    let capture_item = MenuItem::new("Capture", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    let capture_id = capture_item.id().0.clone();
    let quit_id = quit_item.id().0.clone();

    let tray_menu = Menu::new();
    tray_menu.append(&capture_item).expect("Failed to add Capture menu item");
    tray_menu.append(&quit_item).expect("Failed to add Quit menu item");

    let icon = load_tray_icon();

    // Build tray icon — must be kept alive for the lifetime of the app.
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("HydroShot")
        .with_icon(icon)
        .build()
        .expect("Failed to create tray icon");

    tracing::info!("Tray icon created, entering event loop");

    let mut app = App::new(capture_id, quit_id);
    event_loop.run_app(&mut app).expect("Event loop error");
}
