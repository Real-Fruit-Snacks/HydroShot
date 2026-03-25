use crate::capture::CapturedScreen;
use crate::geometry::{Color, Point};
use crate::icons::IconCache;
use crate::overlay::selection::{HitZone, Selection};
use crate::tools::arrow::ArrowTool;
use crate::tools::circle::CircleTool;
use crate::tools::highlight::HighlightTool;
use crate::tools::line::LineTool;
use crate::tools::pencil::PencilTool;
use crate::tools::pixelate::PixelateTool;
use crate::tools::rectangle::RectangleTool;
use crate::tools::step_marker::StepMarkerTool;
use crate::tools::text::TextTool;
use crate::tools::{Annotation, ToolKind};

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
    pub redo_buffer: Vec<Annotation>,
    pub active_tool: ToolKind,
    pub arrow_tool: ArrowTool,
    pub rectangle_tool: RectangleTool,
    pub circle_tool: CircleTool,
    pub line_tool: LineTool,
    pub pencil_tool: PencilTool,
    pub highlight_tool: HighlightTool,
    pub text_tool: TextTool,
    pub pixelate_tool: PixelateTool,
    pub step_marker_tool: StepMarkerTool,
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
    pub icon_cache: IconCache,
}

impl OverlayState {
    pub fn new(screenshot: CapturedScreen, config: &crate::config::Config) -> Self {
        let color = config.default_color();
        let thickness = config.clamped_thickness();

        // Pre-convert screenshot to tiny-skia pixmap format (done once, not per frame)
        let w = screenshot.width;
        let h = screenshot.height;
        let mut screenshot_pixmap = tiny_skia::Pixmap::new(w, h)
            .expect("Failed to create screenshot pixmap");
        {
            let pixels = screenshot_pixmap.data_mut();
            let src = &screenshot.pixels;
            // Bulk convert RGBA to premultiplied — screenshots are fully opaque (a=255)
            // so premultiplication is a no-op, we can just copy bytes directly
            let len = (w * h * 4) as usize;
            if src.len() >= len {
                pixels[..len].copy_from_slice(&src[..len]);
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
            redo_buffer: Vec::new(),
            active_tool: ToolKind::Arrow,
            arrow_tool: ArrowTool::new(color, thickness),
            rectangle_tool: RectangleTool::new(color, thickness),
            circle_tool: CircleTool::new(color, thickness),
            line_tool: LineTool::new(color, thickness),
            pencil_tool: PencilTool::new(color, thickness),
            highlight_tool: HighlightTool::new(color),
            text_tool: TextTool::new(color, 20.0),
            pixelate_tool: PixelateTool::new(10),
            step_marker_tool: StepMarkerTool::new(color, 28.0),
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
            icon_cache: IconCache::new(),
        }
    }
}
