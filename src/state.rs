use crate::capture::CapturedScreen;
use crate::geometry::{Color, Point};
use crate::icons::IconCache;
use crate::overlay::selection::{HitZone, Selection};
use crate::tools::arrow::ArrowTool;
use crate::tools::circle::CircleTool;
use crate::tools::highlight::HighlightTool;
use crate::tools::line::LineTool;
use crate::tools::measurement::MeasurementTool;
use crate::tools::pencil::PencilTool;
use crate::tools::pixelate::PixelateTool;
use crate::tools::rectangle::RectangleTool;
use crate::tools::rounded_rect::RoundedRectTool;
use crate::tools::spotlight::SpotlightTool;
use crate::tools::step_marker::StepMarkerTool;
use crate::tools::text::TextTool;
use crate::tools::{Annotation, ResizeHandle, ToolKind, UndoAction};

pub enum AppState {
    Idle,
    Capturing(Box<OverlayState>),
}

pub struct OverlayState {
    pub screenshot: CapturedScreen,
    /// Pre-converted screenshot pixels in tiny-skia premultiplied format (created once)
    pub screenshot_pixmap: tiny_skia::Pixmap,
    /// Screenshot with dim overlay applied (created once)
    pub dimmed_pixmap: tiny_skia::Pixmap,
    pub selection: Option<Selection>,
    pub annotations: Vec<Annotation>,
    pub undo_stack: Vec<UndoAction>,
    pub redo_stack: Vec<UndoAction>,
    /// Snapshot of an annotation before a drag/resize starts, for undo recording.
    pub pre_drag_annotation: Option<(usize, Annotation)>,
    pub active_tool: ToolKind,
    pub arrow_tool: ArrowTool,
    pub rectangle_tool: RectangleTool,
    pub rounded_rect_tool: RoundedRectTool,
    pub circle_tool: CircleTool,
    pub line_tool: LineTool,
    pub pencil_tool: PencilTool,
    pub highlight_tool: HighlightTool,
    pub text_tool: TextTool,
    pub pixelate_tool: PixelateTool,
    pub step_marker_tool: StepMarkerTool,
    pub spotlight_tool: SpotlightTool,
    pub measurement_tool: MeasurementTool,
    pub text_input_active: bool,
    pub text_input_buffer: String,
    pub text_input_position: Point,
    pub text_input_font_size: f32,
    pub current_color: Color,
    pub current_thickness: f32,
    pub is_selecting: bool,
    pub drag_start: Option<Point>,
    pub drag_zone: Option<HitZone>,
    pub last_mouse_pos: Point,
    pub selected_index: Option<usize>,
    pub select_drag_start: Option<Point>,
    pub resize_handle: Option<ResizeHandle>,
    pub icon_cache: IconCache,
    pub eyedropper_preview: Option<Color>,
    /// In-overlay toast notification (visible on top of the overlay)
    pub toast_message: Option<String>,
    pub toast_until: Option<std::time::Instant>,
    /// Upload confirmation — true means "user confirmed, proceed"
    pub upload_confirmed: bool,
    /// Visible toolbar button indices (original 0-23 indices), computed from ToolbarConfig.
    pub visible_buttons: Vec<usize>,
}

impl OverlayState {
    pub fn new(screenshot: CapturedScreen, config: &crate::config::Config) -> Self {
        let color = config.default_color();
        let thickness = config.clamped_thickness();

        // Pre-convert screenshot to tiny-skia pixmap format (done once, not per frame)
        let w = screenshot.width;
        let h = screenshot.height;
        let mut screenshot_pixmap = match tiny_skia::Pixmap::new(w, h) {
            Some(p) => p,
            None => {
                tracing::warn!("Failed to create {w}x{h} pixmap — falling back to 1x1");
                tiny_skia::Pixmap::new(1, 1).unwrap()
            }
        };
        {
            let pixels = screenshot_pixmap.data_mut();
            let src = &screenshot.pixels;
            // Bulk convert RGBA to premultiplied — screenshots are fully opaque (a=255)
            // so premultiplication is a no-op, we can just copy bytes directly
            let len = (w as usize)
                .checked_mul(h as usize)
                .and_then(|wh| wh.checked_mul(4))
                .unwrap_or(0);
            if len > 0 && src.len() >= len {
                pixels[..len].copy_from_slice(&src[..len]);
            } else if len > 0 {
                tracing::warn!(
                    "Screenshot pixel buffer too small: expected {} bytes, got {}",
                    len,
                    src.len()
                );
            }
        }

        // Pre-compute dimmed version (screenshot + dark overlay, done once)
        let mut dimmed_pixmap = screenshot_pixmap.clone();
        if let Some(rect) = tiny_skia::Rect::from_xywh(0.0, 0.0, w as f32, h as f32) {
            let mut paint = tiny_skia::Paint::default();
            paint.set_color(tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, 0.4).unwrap());
            paint.anti_alias = false;
            dimmed_pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
        }

        Self {
            screenshot,
            screenshot_pixmap,
            dimmed_pixmap,
            selection: None,
            annotations: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pre_drag_annotation: None,
            active_tool: ToolKind::Arrow,
            arrow_tool: ArrowTool::new(color, thickness),
            rectangle_tool: RectangleTool::new(color, thickness),
            rounded_rect_tool: RoundedRectTool::new(color, thickness),
            circle_tool: CircleTool::new(color, thickness),
            line_tool: LineTool::new(color, thickness),
            pencil_tool: PencilTool::new(color, thickness),
            highlight_tool: HighlightTool::new(color),
            text_tool: TextTool::new(color, 20.0),
            pixelate_tool: PixelateTool::new(10),
            step_marker_tool: StepMarkerTool::new(color, 28.0),
            spotlight_tool: SpotlightTool::new(),
            measurement_tool: MeasurementTool::new(color),
            text_input_active: false,
            text_input_buffer: String::new(),
            text_input_position: Point::new(0.0, 0.0),
            text_input_font_size: 20.0,
            current_color: color,
            current_thickness: thickness,
            is_selecting: false,
            drag_start: None,
            drag_zone: None,
            last_mouse_pos: Point::new(0.0, 0.0),
            selected_index: None,
            select_drag_start: None,
            resize_handle: None,
            icon_cache: IconCache::new(),
            eyedropper_preview: None,
            toast_message: None,
            toast_until: None,
            upload_confirmed: false,
            visible_buttons: config.toolbar.visible_button_indices(),
        }
    }

    /// Show an in-overlay toast for the given duration
    pub fn show_toast(&mut self, message: String, duration_ms: u64) {
        self.toast_message = Some(message);
        self.toast_until =
            Some(std::time::Instant::now() + std::time::Duration::from_millis(duration_ms));
    }

    /// Clear expired toast
    pub fn clear_expired_toast(&mut self) {
        if let Some(until) = self.toast_until {
            if std::time::Instant::now() >= until {
                self.toast_message = None;
                self.toast_until = None;
            }
        }
    }
}
