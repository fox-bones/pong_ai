pub mod ball;
pub mod opponent;
pub mod player;
pub mod ui;

use crate::ball::*;
use crate::opponent::*;
use crate::player::*;
use crate::ui::*;
use ::rand::Rng;
use macroquad::audio::{Sound, load_sound, play_sound, stop_sound, play_sound_once, PlaySoundParams};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Pong".to_owned(),
        window_width: 640,
        window_height: 480,
        window_resizable: false,
        ..Default::default()
    }
}

enum GameState {
    StartMenu,
    DifficultyMenu,
    Playing,
    Paused,
    End,
}

fn reset_game(player: &mut Player, opponent: &mut Opponent, ball: &mut Ball) {
    let mut rng = ::rand::thread_rng();
    player.set_start_position();
    opponent.set_start_position();
    if rng.gen_range(0..2) == 0 {
        ball.serve_player();
    } else {
        ball.serve_opponent();
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game_state = GameState::StartMenu;
    let mut difficulty_selection: &str = "easy";
    let mut menu_music_playing: bool = false;

    // Macroquad sound library 
    let player_paddle_hit_sound: Sound = load_sound("resources/paddle_hit_2.wav").await.unwrap();
    let opponent_paddle_hit_sound: Sound = load_sound("resources/paddle_hit.wav").await.unwrap();
    let ui_selection_sound: Sound = load_sound("resources/ui_selection.wav").await.unwrap();
    let player_loses_point_sound: Sound = load_sound("resources/player_loses_point.wav").await.unwrap();
    let player_scores_point_sound: Sound = load_sound("resources/player_scores_point.wav").await.unwrap();
    let menu_music = load_sound("resources/menu_music.wav").await.unwrap();

    let mut stars: Vec<Star> = (0..200).map(|_| create_star()).collect();

    let mut player = Player::new();
    let mut opponent = Opponent::new();
    let mut ball = Ball::new();

    let mut player_score: i32 = 0;
    let mut opponent_score: i32 = 0;

    const WINNING_SCORE: i32 = 11;
    const POINT_FLASH_SECONDS: f64 = 1.0;

    let mut ball_paused_velocity = ball.get_velocity();
    let mut point_flash_ends_at: f64 = 0.0;
    let mut point_scored_by = "player";
    let mut serve_to_player = false;
    let mut waiting_to_serve = false;

    loop {
        clear_background(BLACK);

        match game_state {
            GameState::StartMenu => {
                ui::update_stars(&mut stars, StarDirection::Down);
                ui::draw_start_menu();

                if !menu_music_playing {
                    play_sound(&menu_music, PlaySoundParams { looped: true, volume: 0.5 });
                    menu_music_playing = true;
                }

                if is_key_pressed(KeyCode::Enter) {
                    player_score = 0;
                    opponent_score = 0;
                    waiting_to_serve = false;
                    reset_game(&mut player, &mut opponent, &mut ball);
                    game_state = GameState::DifficultyMenu;
                }
            }
            GameState::DifficultyMenu => {
                ui::update_stars(&mut stars, StarDirection::Diagonal);

                if is_key_pressed(KeyCode::Right) {
                    play_sound_once(&ui_selection_sound);
                    difficulty_selection = "hard";
                }
                if is_key_pressed(KeyCode::Left) {
                    play_sound_once(&ui_selection_sound);
                    difficulty_selection = "easy";
                }
                if is_key_pressed(KeyCode::Enter) {
                    match difficulty_selection {
                        "easy" => opponent.set_difficulty(0.35),
                        "hard" => opponent.set_difficulty(0.7),
                        _ => {}
                    }
                    game_state = GameState::Playing;
                }
                if is_key_pressed(KeyCode::Escape) {
                    game_state = GameState::StartMenu;
                }

                ui::select_difficulty(difficulty_selection);
            }
            GameState::Playing => {
                ui::draw_divider();
                stop_sound(&menu_music);
                menu_music_playing = false;

                player.keyboard_movement();
                player.update_position();
                player.draw();

                opponent.automatic_movement(&ball);
                opponent.update_position();
                opponent.draw();

                // Serve timer — wait for flash to finish before serving
                if waiting_to_serve && get_time() >= point_flash_ends_at {
                    if serve_to_player {
                        ball.serve_player();
                    } else {
                        ball.serve_opponent();
                    }
                    waiting_to_serve = false;
                }

                ball.update();

                if !waiting_to_serve {
                    ball.check_wall_collision();
                    ball.check_player_collision(&player, &player_paddle_hit_sound);
                    ball.check_opponent_collision(&opponent, &opponent_paddle_hit_sound);
                }

                ball.draw();
                ui::draw_score_overlay(player_score, opponent_score);

                if waiting_to_serve {
                    ui::flash_point_screen(point_scored_by);
                }

                if is_key_pressed(KeyCode::Escape) {
                    ball_paused_velocity = ball.get_velocity();
                    game_state = GameState::Paused;
                }

                let ball_x = ball.get_position().x;
                if !waiting_to_serve && ball_x < 0.0 {
                    play_sound_once(&player_loses_point_sound);
                    opponent_score += 1;
                    if opponent_score >= WINNING_SCORE {
                        game_state = GameState::End;
                    } else {
                        point_scored_by = "opponent";
                        serve_to_player = true;
                        point_flash_ends_at = get_time() + POINT_FLASH_SECONDS;
                        waiting_to_serve = true;
                    }
                } else if !waiting_to_serve && ball_x > screen_width() {
                    play_sound_once(&player_scores_point_sound);
                    player_score += 1;
                    if player_score >= WINNING_SCORE {
                        game_state = GameState::End;
                    } else {
                        point_scored_by = "player";
                        serve_to_player = false;
                        point_flash_ends_at = get_time() + POINT_FLASH_SECONDS;
                        waiting_to_serve = true;
                    }
                }
            }
            GameState::Paused => {
                ui::draw_divider();

                player.set_direction("None");
                player.update_position();
                player.draw();

                opponent.set_direction("None");
                opponent.update_position();
                opponent.draw();

                ball.set_velocity_x(0.0);
                ball.set_velocity_y(0.0);
                ball.update();
                ball.draw();

                ui::draw_score_overlay(player_score, opponent_score);
                ui::draw_pause_menu();

                if is_key_pressed(KeyCode::Escape) {
                    ball.set_velocity_x(ball_paused_velocity.x);
                    ball.set_velocity_y(ball_paused_velocity.y);
                    game_state = GameState::Playing;
                }
                if is_key_pressed(KeyCode::Q) {
                    game_state = GameState::StartMenu;
                }
            }
            GameState::End => {
                ui::update_stars(&mut stars, StarDirection::DiagonalReverse);

                if player_score >= WINNING_SCORE {
                    ui::draw_end_menu("player");
                } else {
                    ui::draw_end_menu("opponent");
                }

                if is_key_pressed(KeyCode::Enter) {
                    player_score = 0;
                    opponent_score = 0;
                    waiting_to_serve = false;
                    reset_game(&mut player, &mut opponent, &mut ball);
                    game_state = GameState::Playing;
                }
                if is_key_pressed(KeyCode::Q) {
                    player_score = 0;
                    opponent_score = 0;
                    game_state = GameState::StartMenu;
                }
            }
        }

        next_frame().await;
    }
}