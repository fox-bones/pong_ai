use crate::paddle::{Paddle, bounce_factor};
use crate::sim::{COURT_HEIGHT, COURT_WIDTH, DT, Side};
use macroquad::math::{Vec2, vec2};
use rand::Rng;

pub struct Ball {
    position: Vec2,
    previous_position: Vec2,
    velocity: Vec2,
}

impl Ball {
    pub const RADIUS: f32 = 10.0;

    pub fn new() -> Self {
        Ball {
            position: Vec2::ZERO,
            previous_position: Vec2::ZERO,
            velocity: vec2(400.0, 300.0),
        }
    }

    // Accessors
    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    pub fn get_velocity(&self) -> Vec2 {
        self.velocity
    }

    // Mutators
    /// Puts the ball back in play from the top or bottom of the center line,
    /// heading toward the receiver at a random angle.
    pub fn serve(&mut self, receiver: Side, rng: &mut impl Rng) {
        let x_velocity = match receiver {
            Side::Left  => -150.0,
            Side::Right => 150.0,
        };
        let y_speed = rng.gen_range(150.0..=300.0);

        if rng.gen_bool(0.5) {
            self.position = vec2(COURT_WIDTH / 2.0, Self::RADIUS);
            self.velocity = vec2(x_velocity, y_speed);
        } else {
            self.position = vec2(COURT_WIDTH / 2.0, COURT_HEIGHT - Self::RADIUS);
            self.velocity = vec2(x_velocity, -y_speed);
        }
        self.previous_position = self.position;
    }

    /// Bounces the ball off the top and bottom walls.
    pub fn check_wall_collision(&mut self) {
        if self.position.y <= Self::RADIUS {
            self.position.y = Self::RADIUS;
            self.velocity.y *= -1.0;
        } else if self.position.y >= COURT_HEIGHT - Self::RADIUS {
            self.position.y = COURT_HEIGHT - Self::RADIUS;
            self.velocity.y *= -1.0;
        }
    }

    /// Swept AABB collision with a paddle's face. Returns true if the ball was returned.
    ///
    /// Hitting the center of the paddle diminishes Y velocity;
    /// hitting the edges increases it (see `bounce_factor`).
    pub fn check_paddle_collision(&mut self, paddle: &Paddle) -> bool {
        let paddle_pos = paddle.get_position();

        // The face the ball strikes, and which way along x the ball travels to reach it.
        let (paddle_face, toward) = match paddle.get_side() {
            Side::Left  => (paddle_pos.x + Paddle::WIDTH, -1.0),
            Side::Right => (paddle_pos.x, 1.0),
        };

        let dx = self.position.x - self.previous_position.x;
        if dx.abs() < f32::EPSILON {
            return false;
        }

        // The ball's leading edge, before and after this step.
        let edge_before = self.previous_position.x + toward * Self::RADIUS;
        let edge_after = self.position.x + toward * Self::RADIUS;

        let t = ((paddle_face - edge_before) / dx).clamp(0.0, 1.0);
        let impact_y = self.previous_position.y + (self.position.y - self.previous_position.y) * t;

        let crossed_paddle = toward * (paddle_face - edge_before) > 0.0
            && toward * (edge_after - paddle_face) >= 0.0;

        let y_overlap = impact_y + Self::RADIUS >= paddle_pos.y
            && impact_y - Self::RADIUS <= paddle_pos.y + Paddle::HEIGHT;

        if !(crossed_paddle && y_overlap) {
            return false;
        }

        let value = bounce_factor(impact_y - paddle_pos.y);

        self.position = vec2(paddle_face - toward * Self::RADIUS, impact_y);

        if self.velocity.x.abs() < 300.0 {
            self.velocity.x = toward * 300.0;
        }
        // Vertical speed scales with horizontal speed, so the ball keeps its angle as it speeds
        // up. Otherwise long rallies flatten out until nobody can miss.
        self.velocity.y = -self.velocity.x.abs() * value;
        self.velocity.x *= -1.05;
        true
    }

    pub fn update(&mut self) {
        self.previous_position = self.position;
        self.position += self.velocity * DT;
    }
}
