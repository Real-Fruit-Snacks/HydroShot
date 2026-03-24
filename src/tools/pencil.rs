use super::{Annotation, AnnotationTool};
use crate::geometry::{Color, Point};

pub struct PencilTool {
    color: Color,
    thickness: f32,
    points: Vec<Point>,
    drawing: bool,
}

impl PencilTool {
    pub fn new(color: Color, thickness: f32) -> Self {
        Self {
            color,
            thickness: thickness.clamp(1.0, 20.0),
            points: Vec::new(),
            drawing: false,
        }
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn set_thickness(&mut self, thickness: f32) {
        self.thickness = thickness.clamp(1.0, 20.0);
    }
}

impl AnnotationTool for PencilTool {
    fn on_mouse_down(&mut self, pos: Point) {
        self.points.clear();
        self.points.push(pos);
        self.drawing = true;
    }

    fn on_mouse_move(&mut self, pos: Point) {
        if self.drawing {
            self.points.push(pos);
        }
    }

    fn on_mouse_up(&mut self, pos: Point) -> Option<Annotation> {
        if !self.drawing {
            return None;
        }
        self.points.push(pos);
        self.drawing = false;
        let annotation = Annotation::Pencil {
            points: self.points.clone(),
            color: self.color,
            thickness: self.thickness,
        };
        self.points.clear();
        Some(annotation)
    }

    fn is_drawing(&self) -> bool {
        self.drawing
    }

    fn in_progress_annotation(&self) -> Option<Annotation> {
        if !self.drawing {
            return None;
        }
        Some(Annotation::Pencil {
            points: self.points.clone(),
            color: self.color,
            thickness: self.thickness,
        })
    }
}
