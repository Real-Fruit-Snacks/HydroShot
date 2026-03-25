use super::selection::Selection;
use crate::geometry::Point;

pub const TOOLBAR_HEIGHT: f32 = 40.0;
pub const TOOLBAR_PADDING: f32 = 8.0;
pub const BUTTON_SIZE: f32 = 32.0;
pub const BUTTON_COUNT: usize = 22;

/// Toolbar that appears near the selection rectangle.
/// Buttons: 0=Select, 1=Arrow, 2=Rect, 3=Circle, 4=RoundedRect, 5=Line, 6=Pencil, 7=Highlight, 8=Text, 9=Pixelate, 10=StepMarker, 11=Eyedropper,
///          12-16=colors, 17=OCR, 18=Upload, 19=Pin, 20=Copy, 21=Save
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Toolbar {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Toolbar {
    /// Total width of the toolbar based on button count and padding.
    pub fn toolbar_width() -> f32 {
        TOOLBAR_PADDING + (BUTTON_COUNT as f32) * (BUTTON_SIZE + TOOLBAR_PADDING)
    }

    /// Position the toolbar centered below (or above) the selection.
    pub fn position_for(selection: &Selection, screen_height: f32) -> Self {
        let width = Self::toolbar_width();
        let height = TOOLBAR_HEIGHT;

        // Center horizontally on the selection
        let x = selection.x + (selection.width - width) / 2.0;

        // Place below the selection by default
        let below_y = selection.y + selection.height + TOOLBAR_PADDING;

        // Flip above if toolbar would go off screen bottom
        let y = if below_y + height > screen_height {
            selection.y - height - TOOLBAR_PADDING
        } else {
            below_y
        };

        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Hit-test a point, returning the button index if hit.
    pub fn hit_test(&self, point: Point) -> Option<usize> {
        // Check if point is within toolbar bounds first
        if point.x < self.x
            || point.x > self.x + self.width
            || point.y < self.y
            || point.y > self.y + self.height
        {
            return None;
        }

        for i in 0..BUTTON_COUNT {
            let (bx, by, bw, bh) = self.button_rect(i);
            if point.x >= bx && point.x <= bx + bw && point.y >= by && point.y <= by + bh {
                return Some(i);
            }
        }
        None
    }

    /// Get the rectangle (x, y, width, height) for a button by index.
    pub fn button_rect(&self, index: usize) -> (f32, f32, f32, f32) {
        let bx = self.x + TOOLBAR_PADDING + (index as f32) * (BUTTON_SIZE + TOOLBAR_PADDING);
        let by = self.y + (self.height - BUTTON_SIZE) / 2.0;
        (bx, by, BUTTON_SIZE, BUTTON_SIZE)
    }
}
