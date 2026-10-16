use crate::opponent::*;
use crate::player::*;
use ::rand::Rng;
use macroquad::audio::{Sound, play_sound_once};
use macroquad::prelude::*;

pub struct Ball {
    radius: f32,
    position: Vec2,
    previous_position: Vec2,
    velocity: Vec2,
}

impl Ball {
    pub fn new() -> Self {
        Ball {
            radius: 10.0,
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

    pub fn get_velocity_x(&self) -> f32 {
        self.velocity.x
    }

    pub fn get_velocity_y(&self) -> f32 {
        self.velocity.y
    }

    pub fn get_radius(&self) -> f32 {
        self.radius
    }

    // Mutators
    pub fn set_velocity_x(&mut self, velocity: f32) {
        self.velocity.x = velocity;
    }

    pub fn set_velocity_y(&mut self, velocity: f32) {
        self.velocity.y = velocity;
    }

    pub fn set_position(&mut self, position: Vec2) {
        self.position = position;
        self.previous_position = position;
    }

    pub fn serve_opponent(&mut self) {
        let mut rng = ::rand::thread_rng();
        let x_dir = if rng.gen_bool(0.5) { 150.0 } else { -150.0 };
        self.position = vec2(screen_width() / 2.0, self.radius);
        self.velocity = vec2(x_dir, -300.0);
        self.previous_position = self.position;
    }

    pub fn serve_player(&mut self) {
        let mut rng = ::rand::thread_rng();
        if rng.gen_bool(0.5) {
            self.position = vec2(screen_width() / 2.0, self.radius);
            self.velocity = vec2(-150.0, -300.0);
        } else {
            self.position = vec2(screen_width() / 2.0, screen_height() - self.radius);
            self.velocity = vec2(-150.0, 300.0);
        }
        self.previous_position = self.position;
    }

    /// Bounces the ball off the top and bottom walls.
    pub fn check_wall_collision(&mut self) {
        if self.position.y <= self.radius {
            self.position.y = self.radius;
            self.velocity.y *= -1.0;
        } else if self.position.y >= screen_height() - self.radius {
            self.position.y = screen_height() - self.radius;
            self.velocity.y *= -1.0;
        }
    }

    /// Swept AABB collision with the player paddle.
    ///
    /// Hitting the center of the paddle diminishes Y velocity;
    /// hitting the edges increases it, based on a 100-segment gradient.
    pub fn check_player_collision(&mut self, player: &Player, paddle_hit_sound: &Sound) {
        let player_pos = player.get_position();
        let paddle_face = player_pos.x + player.get_width();

        let dx = self.position.x - self.previous_position.x;
        if dx.abs() < f32::EPSILON {
            return;
        }

        let t = ((paddle_face - (self.previous_position.x - self.radius)) / dx).clamp(0.0, 1.0);
        let impact_y = self.previous_position.y + (self.position.y - self.previous_position.y) * t;

        let crossed_paddle = self.previous_position.x - self.radius > paddle_face
            && self.position.x - self.radius <= paddle_face;

        let y_overlap = impact_y + self.radius >= player_pos.y
            && impact_y - self.radius <= player_pos.y + player.get_height();

        if !(crossed_paddle && y_overlap) {
            return;
        }

        play_sound_once(paddle_hit_sound);

        let value = gradient_value(player.get_gradient(), player.get_height(), impact_y - player_pos.y);

        self.position = vec2(paddle_face + self.radius, impact_y);

        if self.velocity.x.abs() < 300.0 {
            self.velocity.x = -300.0;
        }
        self.velocity.x *= -1.05;
        self.velocity.y = -300.0 * value;
    }

    /// Swept AABB collision with the opponent paddle.
    pub fn check_opponent_collision(&mut self, opponent: &Opponent, paddle_hit_sound: &Sound) {
        let opponent_pos = opponent.get_position();
        let paddle_face = opponent_pos.x;

        let dx = self.position.x - self.previous_position.x;
        if dx.abs() < f32::EPSILON {
            return;
        }

        let t = ((paddle_face - (self.previous_position.x + self.radius)) / dx).clamp(0.0, 1.0);
        let impact_y = self.previous_position.y + (self.position.y - self.previous_position.y) * t;

        let crossed_paddle = self.previous_position.x + self.radius < paddle_face
            && self.position.x + self.radius >= paddle_face;

        let y_overlap = impact_y + self.radius >= opponent_pos.y
            && impact_y - self.radius <= opponent_pos.y + opponent.get_height();

        if !(crossed_paddle && y_overlap) {
            return;
        }

        play_sound_once(paddle_hit_sound);

        let value = gradient_value(opponent.get_gradient(), opponent.get_height(), impact_y - opponent_pos.y);

        self.position = vec2(paddle_face - self.radius, impact_y);

        if self.velocity.x.abs() < 300.0 {
            self.velocity.x = 300.0;
        }
        self.velocity.x *= -1.05;
        self.velocity.y = -300.0 * value;
    }

    pub fn update(&mut self) {
        self.previous_position = self.position;
        self.position += self.velocity * get_frame_time();
    }

    pub fn draw(&self) {
        draw_circle(self.position.x, self.position.y, self.radius, GREEN);
    }
}

/// Returns the gradient multiplier for a given relative hit position on a paddle.
fn gradient_value(gradient: &Vec<(f32, f32)>, paddle_height: f32, relative_y: f32) -> f32 {
    const SEGMENTS: usize = 100;
    let segment_height = paddle_height / SEGMENTS as f32;
    let index = ((relative_y / segment_height) as usize).min(SEGMENTS - 1);
    gradient[index].1
}