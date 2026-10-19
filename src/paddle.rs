use crate::sim::{COURT_HEIGHT, COURT_WIDTH, DT, Side};
use macroquad::math::{Vec2, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    None,
    Up,
    Down,
}

pub struct Paddle {
    side: Side,
    position: Vec2,
}

impl Paddle {
    pub const HEIGHT: f32 = 100.0;
    pub const WIDTH: f32 = 8.0;
    pub const SPEED: f32 = 300.0;
    /// Top edge of a vertically centered paddle.
    pub const START_Y: f32 = COURT_HEIGHT / 2.0 - Self::HEIGHT / 2.0;
    /// Gap between the paddle and its edge of the court.
    const MARGIN: f32 = 10.0;

    pub fn new(side: Side) -> Self {
        let mut paddle = Paddle {
            side,
            position: Vec2::ZERO,
        };
        paddle.set_start_position();
        paddle
    }

    // Accessors
    pub fn get_side(&self) -> Side {
        self.side
    }

    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    // Mutators
    /// Places the paddle at its own edge of the court, vertically centered.
    pub fn set_start_position(&mut self) {
        let x = match self.side {
            Side::Left  => Self::MARGIN,
            Side::Right => COURT_WIDTH - (Self::WIDTH + Self::MARGIN),
        };
        self.position = vec2(x, Self::START_Y);
    }

    /// Moves one step in the given direction, stopping at the walls.
    pub fn update_position(&mut self, direction: Direction) {
        match direction {
            Direction::Up   => self.position.y -= Self::SPEED * DT,
            Direction::Down => self.position.y += Self::SPEED * DT,
            Direction::None => {}
        }
        self.position.y = self.position.y.clamp(0.0, COURT_HEIGHT - Self::HEIGHT);
    }
}

/// Returns the bounce multiplier for a ball striking `offset` pixels below a paddle's top edge.
///
/// The paddle is split into 100 segments: the top half spreads 1.0 → 0.01, the bottom half
/// -0.1 → -1.0. Center hits leave nearly flat; edge hits leave steeply.
pub fn bounce_factor(offset: f32) -> f32 {
    const SEGMENTS: usize = 100;
    let segment_height = Paddle::HEIGHT / SEGMENTS as f32;
    let i = ((offset / segment_height) as usize).min(SEGMENTS - 1);

    if i < 50 {
        let t = i as f32 / 49.0;
        1.0 + (0.01 - 1.0) * t
    } else {
        let t = (i - 50) as f32 / 49.0;
        -0.1 + (-1.0 - -0.1) * t
    }
}
