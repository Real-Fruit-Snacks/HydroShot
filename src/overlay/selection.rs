use crate::geometry::Point;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HitZone {
    Inside,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Selection {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Selection {
    /// Create a selection from two points, normalizing so x/y is the minimum.
    pub fn from_points(a: Point, b: Point) -> Self {
        let x = a.x.min(b.x);
        let y = a.y.min(b.y);
        let width = (a.x - b.x).abs();
        let height = (a.y - b.y).abs();
        Self { x, y, width, height }
    }

    /// Check if a point is inside the selection rectangle.
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x
            && p.x <= self.x + self.width
            && p.y >= self.y
            && p.y <= self.y + self.height
    }

    /// Move the selection by a delta.
    pub fn move_by(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
    }

    /// Hit-test a point against the selection, returning which zone it falls in.
    /// `zone_size` is the width of the edge/corner grab zones (typically 8px).
    /// Corners take priority over edges, edges over inside, None if outside.
    pub fn hit_test(&self, p: Point, zone_size: f32) -> Option<HitZone> {
        let left = self.x;
        let right = self.x + self.width;
        let top = self.y;
        let bottom = self.y + self.height;

        // Check if point is within the extended bounds (including zone_size border)
        let in_x = p.x >= left - zone_size && p.x <= right + zone_size;
        let in_y = p.y >= top - zone_size && p.y <= bottom + zone_size;
        if !in_x || !in_y {
            return None;
        }

        let near_left = (p.x - left).abs() <= zone_size;
        let near_right = (p.x - right).abs() <= zone_size;
        let near_top = (p.y - top).abs() <= zone_size;
        let near_bottom = (p.y - bottom).abs() <= zone_size;

        // Corners first
        if near_left && near_top {
            return Some(HitZone::TopLeft);
        }
        if near_right && near_top {
            return Some(HitZone::TopRight);
        }
        if near_left && near_bottom {
            return Some(HitZone::BottomLeft);
        }
        if near_right && near_bottom {
            return Some(HitZone::BottomRight);
        }

        // Edges
        if near_top {
            return Some(HitZone::Top);
        }
        if near_bottom {
            return Some(HitZone::Bottom);
        }
        if near_left {
            return Some(HitZone::Left);
        }
        if near_right {
            return Some(HitZone::Right);
        }

        // Inside
        if self.contains(p) {
            return Some(HitZone::Inside);
        }

        None
    }

    /// Resize the selection by dragging a zone handle by (dx, dy).
    pub fn resize(&mut self, zone: HitZone, dx: f32, dy: f32) {
        match zone {
            HitZone::TopLeft => {
                self.x += dx;
                self.y += dy;
                self.width -= dx;
                self.height -= dy;
            }
            HitZone::TopRight => {
                self.y += dy;
                self.width += dx;
                self.height -= dy;
            }
            HitZone::BottomLeft => {
                self.x += dx;
                self.width -= dx;
                self.height += dy;
            }
            HitZone::BottomRight => {
                self.width += dx;
                self.height += dy;
            }
            HitZone::Top => {
                self.y += dy;
                self.height -= dy;
            }
            HitZone::Bottom => {
                self.height += dy;
            }
            HitZone::Left => {
                self.x += dx;
                self.width -= dx;
            }
            HitZone::Right => {
                self.width += dx;
            }
            HitZone::Inside => {
                // Moving, not resizing — no-op here; use move_by instead
            }
        }
    }
}
