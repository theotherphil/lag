/// A location within a 2D grid.
/// The origin is at the top left of the grid.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub max_x: usize,
    pub max_y: usize,
    pub x: usize,
    pub y: usize,
}

impl Cursor {
    /// `max_x` and `max_y` are inclusive.
    pub fn new(max_x: usize, max_y: usize) -> Cursor {
        Cursor {
            max_x,
            max_y,
            x: 0,
            y: 0,
        }
    }

    /// A positive `delta` moves right.
    pub fn move_x(&mut self, delta: isize) {
        if delta > 0 {
            self.x += (delta as usize).min(self.max_x - self.x);
        } else {
            self.x -= (-delta as usize).min(self.x);
        }
    }

    /// A positive `delta` moves down.
    pub fn move_y(&mut self, delta: isize) {
        if delta > 0 {
            self.y += (delta as usize).min(self.max_y - self.y);
        } else {
            self.y -= (-delta as usize).min(self.y);
        }
    }

    pub fn move_to_left_boundary(&mut self) {
        self.x = 0;
    }

    pub fn move_to_right_boundary(&mut self) {
        self.x = self.max_x;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location(cursor: &Cursor) -> (usize, usize) {
        (cursor.x, cursor.y)
    }

    #[test]
    fn cursor_movement() {
        let (max_x, max_y) = (10, 15);
        let mut cursor = Cursor::new(max_x, max_y);
        assert_eq!(location(&cursor), (0, 0));
        cursor.move_x(5);
        assert_eq!(location(&cursor), (5, 0));
        cursor.move_x(7);
        assert_eq!(location(&cursor), (10, 0));
        cursor.move_x(-12);
        assert_eq!(location(&cursor), (0, 0));
        cursor.move_y(7);
        assert_eq!(location(&cursor), (0, 7));
        cursor.move_y(8);
        assert_eq!(location(&cursor), (0, 15));
        cursor.move_y(1);
        assert_eq!(location(&cursor), (0, 15));
        cursor.move_y(-20);
        assert_eq!(location(&cursor), (0, 0));
    }
}
