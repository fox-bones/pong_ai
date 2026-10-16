use crate::ball::*;
use macroquad::prelude::*;

pub enum Direction {
    None,
    Up,
    Down,
}

pub struct Opponent {
    height: f32,
    width: f32,
    speed: f32,
    direction: Direction,
    position: Vec2,
    gradient: Vec<(f32, f32)>,
    target_y: f32,
    difficulty: f32,
}

impl Opponent {
    pub fn new() -> Self {
        let mut opponent = Opponent {
            height: 100.0,
            width: 8.0,
            speed: 300.0,
            direction: Direction::None,
            position: Vec2::ZERO,
            gradient: vec![],
            target_y: 0.0,
            difficulty: 0.85,
        };
        opponent.create_gradient();
        opponent
    }

    // Accessors
    pub fn get_position(&self) -> &Vec2 {
        &self.position
    }

    pub fn get_height(&self) -> f32 {
        self.height
    }

    pub fn get_width(&self) -> f32 {
        self.width
    }

    pub fn get_gradient(&self) -> &Vec<(f32, f32)> {
        &self.gradient
    }

    // Mutators
    pub fn set_direction(&mut self, direction: &str) {
        match direction {
            "None" => self.direction = Direction::None,
            "Up"   => self.direction = Direction::Up,
            "Down" => self.direction = Direction::Down,
            _      => {}
        }
    }

    pub fn set_difficulty(&mut self, difficulty: f32) {
        self.difficulty = difficulty;
    }

    /// Creates a spread from 1.0 → 0.01 (top half) then -0.1 → -1.0 (bottom half),
    /// pasted to the paddle for dynamic velocity changes based on where the ball hits.
    pub fn create_gradient(&mut self) {
        self.gradient.clear();

        let segments = 100;
        let segment_height = self.height / segments as f32;

        for i in 0..segments {
            let y_offset = i as f32 * segment_height;

            let value = if i < 50 {
                let t = i as f32 / 49.0;
                1.0 + (0.01 - 1.0) * t
            } else {
                let t = (i - 50) as f32 / 49.0;
                -0.1 + (-1.0 - -0.1) * t
            };

            self.gradient.push((y_offset, value));
        }
    }

    /// Sets starting position at the right side, vertically centered.
    pub fn set_start_position(&mut self) {
        self.position.x = screen_width() - (self.width + 10.0);
        self.position.y = (screen_height() / 2.0) - (self.height / 2.0);
        self.target_y = self.position.y;
    }

    pub fn keyboard_movement(&mut self) {
        if is_key_down(KeyCode::Up) && self.position.y > 0.0 {
            self.direction = Direction::Up;
        } else if is_key_down(KeyCode::Down) && (self.position.y + self.height) < screen_height() {
            self.direction = Direction::Down;
        } else {
            self.direction = Direction::None;
        }
    }

    /// Automatic movement — tracks the ball when it's heading toward the opponent's side.
    pub fn automatic_movement(&mut self, ball: &Ball) {
        let ball_pos = ball.get_position();
        let ball_vel_x = ball.get_velocity_x();

        if ball_vel_x > 0.0 && ball_pos.x >= screen_width() * (1.0 - self.difficulty) {
            let paddle_center = self.position.y + self.height / 2.0;
            let ball_center_y = ball_pos.y;

            // Dead zone: don't react if already close enough (kills jitter)
            let dead_zone = self.height * (1.0 - self.difficulty) * 0.3 + 4.0;
            if (paddle_center - ball_center_y).abs() > dead_zone {
                // Lerp target_y toward ball — smooth, not instant
                let lerp_speed = self.difficulty * 8.0;
                self.target_y += (ball_center_y - self.height / 2.0 - self.target_y)
                    * (lerp_speed * get_frame_time()).min(1.0);
            }
        } else {
            // Ball moving away: drift back toward center lazily
            let center = screen_height() / 2.0 - self.height / 2.0;
            self.target_y += (center - self.target_y) * (2.0 * get_frame_time()).min(1.0);
        }

        let diff = self.target_y - self.position.y;

        if diff > 2.0 && self.position.y + self.height < screen_height() {
            self.direction = Direction::Down;
        } else if diff < -2.0 && self.position.y > 0.0 {
            self.direction = Direction::Up;
        } else {
            self.direction = Direction::None;
        }
    }

    pub fn draw(&self) {
        draw_rectangle(self.position.x, self.position.y, self.width, self.height, GREEN);
    }

    pub fn update_position(&mut self) {
        match self.direction {
            Direction::Up   => self.position.y -= self.speed * get_frame_time(),
            Direction::Down => self.position.y += self.speed * get_frame_time(),
            Direction::None => {}
        }
        self.position.y = self.position.y.clamp(0.0, screen_height() - self.height);
    }
}