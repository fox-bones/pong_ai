use macroquad::prelude::*;

pub struct Star {
    pos: Vec2,
    size: f32,
    speed: f32,
    phase: f32,
    color: Color,
}

pub enum StarDirection {
    Down,
    Diagonal,
    DiagonalReverse,
}

pub fn create_star() -> Star {
    let color = match rand::gen_range(0, 3) {
        0 => WHITE,
        1 => LIGHTGRAY,
        _ => GRAY,
    };

    let (size, speed) = match rand::gen_range(0, 3) {
        0 => (1.0, 5.0),
        1 => (2.0, 15.0),
        _ => (3.0, 30.0),
    };

    Star {
        pos: vec2(
            rand::gen_range(0.0, screen_width()),
            rand::gen_range(0.0, screen_height()),
        ),
        size,
        speed,
        phase: rand::gen_range(0.0, std::f32::consts::TAU),
        color,
    }
}

pub fn update_stars(stars: &mut Vec<Star>, direction: StarDirection) {
    let dt = get_frame_time();

    for star in stars.iter_mut() {
        let alpha = 0.3 + 0.7 * ((get_time() as f32 + star.phase).sin() + 1.0) * 0.5;
        let step = (star.speed * dt) / 2.0;

        match direction {
            StarDirection::Down => {
                star.pos.y += step;
            }
            StarDirection::Diagonal => {
                star.pos.x += step;
                star.pos.y += step;
            }
            StarDirection::DiagonalReverse => {
                star.pos.x -= step;
                star.pos.y -= step;
            }
        }

        draw_rectangle(
            star.pos.x,
            star.pos.y,
            star.size,
            star.size,
            Color { a: alpha, ..star.color },
        );

        // Wrap stars around screen edges
        if star.pos.y > screen_height() {
            star.pos.y = 0.0;
            star.pos.x = rand::gen_range(0.0, screen_width());
            star.phase = rand::gen_range(0.0, std::f32::consts::TAU);
        }
        if star.pos.x > screen_width() {
            star.pos.x = 0.0;
            star.pos.y = rand::gen_range(0.0, screen_height());
            star.phase = rand::gen_range(0.0, std::f32::consts::TAU);
        }
        if star.pos.x < 0.0 {
            star.pos.x = screen_width();
            star.pos.y = rand::gen_range(0.0, screen_height());
            star.phase = rand::gen_range(0.0, std::f32::consts::TAU);
        }
        if star.pos.y < 0.0 {
            star.pos.y = screen_height();
            star.pos.x = rand::gen_range(0.0, screen_width());
            star.phase = rand::gen_range(0.0, std::f32::consts::TAU);
        }
    }
}

/// Draws text centered horizontally at the given Y position.
fn draw_centered_text(text: &str, y: f32, size: f32, color: Color) {
    let dimensions = measure_text(text, None, size as u16, 1.0);
    draw_text(
        text,
        screen_width() / 2.0 - dimensions.width / 2.0,
        y,
        size,
        color,
    );
}

pub fn draw_start_menu() {
    draw_centered_text("Pong", 230.0, 48.0, WHITE);
    draw_centered_text("Press Enter to Start", 270.0, 28.0, WHITE);
}

pub fn draw_pause_menu() {
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.6));
    draw_centered_text("Paused", 210.0, 48.0, YELLOW);
    draw_centered_text("Esc - Resume", 250.0, 28.0, WHITE);
    draw_centered_text("Q - Main Menu", 290.0, 28.0, WHITE);
}

pub fn draw_divider() {
    draw_rectangle(screen_width() / 2.0, 0.0, 1.0, screen_height(), GRAY);
}

pub fn select_difficulty(difficulty: &str) {
    let difficulty_dims = measure_text("Select Difficulty", None, 48, 1.0);
    let easy_dims = measure_text("Easy", None, 28, 1.0);
    let hard_dims = measure_text("Hard", None, 28, 1.0);

    let easy_x = (screen_width() / 2.0) - (difficulty_dims.width / 4.0) - (easy_dims.width / 2.0);
    let hard_x = (screen_width() / 2.0) + (difficulty_dims.width / 4.0) - (hard_dims.width / 2.0);
    let show_arrow = get_time() % 1.0 < 0.5;

    draw_centered_text("Select Difficulty", 230.0, 48.0, YELLOW);
    draw_text("Easy", easy_x, 270.0, 28.0, WHITE);
    draw_text("Hard", hard_x, 270.0, 28.0, WHITE);

    if show_arrow {
        let arrow_x = if difficulty == "easy" { easy_x } else { hard_x };
        draw_text(">", arrow_x - 20.0, 270.0, 28.0, WHITE);
    }
}

pub fn draw_score_overlay(player_score: i32, opponent_score: i32) {
    let text = format!("{}     {}", player_score, opponent_score);
    let size = 42.0;
    let dimensions = measure_text(&text, None, size as u16, 1.0);

    draw_text(
        &text,
        screen_width() / 2.0 - dimensions.width / 2.0,
        62.0,
        size,
        Color::new(1.0, 1.0, 1.0, 0.55),
    );
}

pub fn flash_point_screen(side: &str) {
    let text = "+1";
    let size = 56.0;
    let dimensions = measure_text(text, None, size as u16, 1.0);

    let (dim_x, highlight_x, text_x) = if side == "opponent" {
        (0.0, screen_width() / 2.0, screen_width() * 0.75)
    } else {
        (screen_width() / 2.0, 0.0, screen_width() * 0.25)
    };

    draw_rectangle(dim_x, 0.0, screen_width() / 2.0, screen_height(), Color::new(0.0, 0.0, 0.0, 0.5));
    draw_rectangle(highlight_x, 0.0, screen_width() / 2.0, screen_height(), Color::new(255.0, 255.0, 255.0, 0.1));
    draw_text(text, text_x - dimensions.width / 2.0, screen_height() / 2.0, size, WHITE);
}

pub fn draw_end_menu(winner: &str) {
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.6));

    let title = if winner == "player" { "Player Won" } else { "Opponent Won" };
    draw_centered_text(title, 210.0, 48.0, YELLOW);
    draw_centered_text("Enter - Restart", 250.0, 28.0, WHITE);
    draw_centered_text("Q - Main Menu", 290.0, 28.0, WHITE);
}