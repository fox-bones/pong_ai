mod ui;

use crate::ui::*;
use macroquad::audio::{Sound, load_sound, play_sound, stop_sound, play_sound_once, PlaySoundParams};
use macroquad::prelude::*;
use pong::brain::Brain;
use pong::controller::{Bot, Controller, Observation, REACTION_TIME};
use pong::paddle::Direction;
use pong::sim::{COURT_HEIGHT, COURT_WIDTH, DT, Side, Sim};

fn window_conf() -> Conf {
    Conf {
        window_title: "Pong".to_owned(),
        window_width: COURT_WIDTH as i32,
        window_height: COURT_HEIGHT as i32,
        window_resizable: false,
        ..Default::default()
    }
}

/// Most time the sim catches up on in one frame, so a stall (like dragging the
/// window) doesn't fast-forward the match.
const MAX_FRAME_TIME: f32 = 0.1;

/// Easy is the same trained bot with slower reactions (Hard and Watch use `REACTION_TIME`,
/// the speed it trained at). Raise this to make Easy easier.
const EASY_REACTION_TIME: f32 = 0.45;

enum GameState {
    StartMenu,
    DifficultyMenu,
    Playing,
    Paused,
    End,
}

/// The choices on the difficulty menu.
#[derive(Clone, Copy)]
enum Mode {
    Easy,
    Hard,
    /// The trained bot plays itself.
    Watch,
}

impl Mode {
    const ALL: [Mode; 3] = [Mode::Easy, Mode::Hard, Mode::Watch];

    fn label(self) -> &'static str {
        match self {
            Mode::Easy => "Easy",
            Mode::Hard => "Hard",
            Mode::Watch => "Watch",
        }
    }
}

/// The human player: the arrow keys move the paddle.
struct KeyboardController;

impl Controller for KeyboardController {
    fn act(&mut self, _observation: &Observation) -> Direction {
        if is_key_down(KeyCode::Up) {
            Direction::Up
        } else if is_key_down(KeyCode::Down) {
            Direction::Down
        } else {
            Direction::None
        }
    }
}

/// The left and right paddles' controllers for a mode.
fn controllers(mode: Mode, brain: &Brain) -> (Box<dyn Controller>, Box<dyn Controller>) {
    match mode {
        Mode::Easy => (Box::new(KeyboardController), Box::new(Bot::new(brain.clone(), EASY_REACTION_TIME))),
        Mode::Hard => (Box::new(KeyboardController), Box::new(Bot::new(brain.clone(), REACTION_TIME))),
        Mode::Watch => (Box::new(Bot::new(brain.clone(), REACTION_TIME)), Box::new(Bot::new(brain.clone(), REACTION_TIME))),
    }
}

/// Loads the brain saved by the trainer.
async fn load_brain(path: &str) -> Brain {
    let text = load_string(path).await.unwrap_or_else(|_| {
        panic!("{path} is missing; train the bot with `cargo run --release --bin train`")
    });
    Brain::from_text(&text).unwrap_or_else(|err| panic!("{path}: {err}"))
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game_state = GameState::StartMenu;
    let mut selected_mode: usize = 0;
    let mut mode = Mode::Easy;
    let mut menu_music_playing: bool = false;

    // Macroquad sound library
    let player_paddle_hit_sound: Sound = load_sound("resources/paddle_hit_2.wav").await.unwrap();
    let opponent_paddle_hit_sound: Sound = load_sound("resources/paddle_hit.wav").await.unwrap();
    let ui_selection_sound: Sound = load_sound("resources/ui_selection.wav").await.unwrap();
    let player_loses_point_sound: Sound = load_sound("resources/player_loses_point.wav").await.unwrap();
    let player_scores_point_sound: Sound = load_sound("resources/player_scores_point.wav").await.unwrap();
    let menu_music = load_sound("resources/menu_music.wav").await.unwrap();

    let mut stars: Vec<Star> = (0..200).map(|_| create_star()).collect();

    let brain = load_brain("resources/brains/champion.txt").await;

    // The player is on the left, the opponent on the right.
    let mut sim = Sim::new(::rand::random());
    let (mut left, mut right) = controllers(mode, &brain);

    // Frame time the sim hasn't caught up on yet; it only advances in whole DT steps.
    let mut unsimulated_time: f32 = 0.0;

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
                    sim.reset();
                    game_state = GameState::DifficultyMenu;
                }
            }
            GameState::DifficultyMenu => {
                ui::update_stars(&mut stars, StarDirection::Diagonal);

                if is_key_pressed(KeyCode::Right) {
                    play_sound_once(&ui_selection_sound);
                    selected_mode = (selected_mode + 1).min(Mode::ALL.len() - 1);
                }
                if is_key_pressed(KeyCode::Left) {
                    play_sound_once(&ui_selection_sound);
                    selected_mode = selected_mode.saturating_sub(1);
                }
                if is_key_pressed(KeyCode::Enter) {
                    mode = Mode::ALL[selected_mode];
                    (left, right) = controllers(mode, &brain);
                    game_state = GameState::Playing;
                }
                if is_key_pressed(KeyCode::Escape) {
                    game_state = GameState::StartMenu;
                }

                ui::select_difficulty(&Mode::ALL.map(Mode::label), selected_mode);
            }
            GameState::Playing => {
                stop_sound(&menu_music);
                menu_music_playing = false;

                unsimulated_time += get_frame_time().min(MAX_FRAME_TIME);
                while unsimulated_time >= DT {
                    unsimulated_time -= DT;

                    let left_move = left.act(&sim.observe(Side::Left));
                    let right_move = right.act(&sim.observe(Side::Right));
                    let events = sim.step(left_move, right_move);

                    match events.paddle_hit {
                        Some(Side::Left) => play_sound_once(&player_paddle_hit_sound),
                        Some(Side::Right) => play_sound_once(&opponent_paddle_hit_sound),
                        None => {}
                    }
                    match events.point {
                        Some(Side::Left) => play_sound_once(&player_scores_point_sound),
                        Some(Side::Right) => play_sound_once(&player_loses_point_sound),
                        None => {}
                    }
                }

                ui::draw_match(&sim);
                if let Some(scorer) = sim.get_point_scorer() {
                    ui::flash_point_screen(scorer);
                }

                if sim.get_winner().is_some() {
                    game_state = GameState::End;
                } else if is_key_pressed(KeyCode::Escape) {
                    game_state = GameState::Paused;
                }
            }
            GameState::Paused => {
                ui::draw_match(&sim);
                ui::draw_pause_menu();

                if is_key_pressed(KeyCode::Escape) {
                    game_state = GameState::Playing;
                }
                if is_key_pressed(KeyCode::Q) {
                    game_state = GameState::StartMenu;
                }
            }
            GameState::End => {
                ui::update_stars(&mut stars, StarDirection::DiagonalReverse);

                if let Some(winner) = sim.get_winner() {
                    let title = match (mode, winner) {
                        (Mode::Watch, Side::Left) => "Left Won",
                        (Mode::Watch, Side::Right) => "Right Won",
                        (_, Side::Left) => "Player Won",
                        (_, Side::Right) => "Opponent Won",
                    };
                    ui::draw_end_menu(title);
                }

                if is_key_pressed(KeyCode::Enter) {
                    sim.reset();
                    left.reset();
                    right.reset();
                    game_state = GameState::Playing;
                }
                if is_key_pressed(KeyCode::Q) {
                    game_state = GameState::StartMenu;
                }
            }
        }

        next_frame().await;
    }
}
