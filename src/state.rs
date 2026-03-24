use crate::capture::CapturedScreen;
use crate::geometry::{Color, Point};
use crate::overlay::selection::{HitZone, Selection};
use crate::tools::arrow::ArrowTool;
use crate::tools::pencil::PencilTool;
use crate::tools::rectangle::RectangleTool;
use crate::tools::text::TextTool;
use crate::tools::{Annotation, ToolKind};

pub enum AppState {
    Idle,
    Capturing(Box<OverlayState>),
}

pub struct OverlayState {
    pub screenshot: CapturedScreen,
    pub selection: Option<Selection>,
    pub annotations: Vec<Annotation>,
    pub redo_buffer: Vec<Annotation>,
    pub active_tool: ToolKind,
    pub arrow_tool: ArrowTool,
    pub rectangle_tool: RectangleTool,
    pub pencil_tool: PencilTool,
    pub text_tool: TextTool,
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
}

impl OverlayState {
    pub fn new(screenshot: CapturedScreen) -> Self {
        let color = Color::red();
        Self {
            screenshot,
            selection: None,
            annotations: Vec::new(),
            redo_buffer: Vec::new(),
            active_tool: ToolKind::Arrow,
            arrow_tool: ArrowTool::new(color, 3.0),
            rectangle_tool: RectangleTool::new(color, 3.0),
            pencil_tool: PencilTool::new(color, 3.0),
            text_tool: TextTool::new(color, 20.0),
            text_input_active: false,
            text_input_buffer: String::new(),
            text_input_position: Point::new(0.0, 0.0),
            text_input_font_size: 20.0,
            current_color: color,
            current_thickness: 3.0,
            is_selecting: false,
            drag_start: None,
            drag_zone: None,
            last_mouse_pos: Point::new(0.0, 0.0),
        }
    }
}
