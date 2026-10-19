use crate::ball::Ball;
use crate::controller::Observation;
use crate::paddle::{Direction, Paddle};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub const COURT_WIDTH: f32 = 640.0;
pub const COURT_HEIGHT: f32 = 480.0;

/// The sim always advances in steps of this length, whatever the frame rate.
/// 120 steps per second divides evenly into both 60 Hz and 120 Hz displays.
pub const STEPS_PER_SECOND: u32 = 120;
pub const DT: f32 = 1.0 / STEPS_PER_SECOND as f32;

pub const WINNING_SCORE: u32 = 11;

/// Pause after a point before the next serve, while the "+1" flash shows.
const SERVE_DELAY_STEPS: u32 = STEPS_PER_SECOND;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// What happened during one step, so the game can play sounds and a trainer can hand out rewards.
#[derive(Clone, Copy, Debug, Default)]
pub struct StepEvents {
    /// The paddle that returned the ball, if one did.
    pub paddle_hit: Option<Side>,
    /// The side that scored, if the ball went out.
    pub point: Option<Side>,
}

/// After a point: who scored, and how long until the next serve.
struct ServeDelay {
    scored_by: Side,
    steps_left: u32,
}

/// One match of Pong, advanced a fixed step at a time.
///
/// The same seed and the same paddle moves always replay the same match.
pub struct Sim {
    left: Paddle,
    right: Paddle,
    ball: Ball,
    left_score: u32,
    right_score: u32,
    serve_delay: Option<ServeDelay>,
    winner: Option<Side>,
    rng: StdRng,
}

impl Sim {
    pub fn new(seed: u64) -> Self {
        let mut sim = Sim {
            left: Paddle::new(Side::Left),
            right: Paddle::new(Side::Right),
            ball: Ball::new(),
            left_score: 0,
            right_score: 0,
            serve_delay: None,
            winner: None,
            rng: StdRng::seed_from_u64(seed),
        };
        sim.reset();
        sim
    }

    // Accessors
    pub fn get_paddle(&self, side: Side) -> &Paddle {
        match side {
            Side::Left => &self.left,
            Side::Right => &self.right,
        }
    }

    pub fn get_ball(&self) -> &Ball {
        &self.ball
    }

    pub fn get_score(&self, side: Side) -> u32 {
        match side {
            Side::Left => self.left_score,
            Side::Right => self.right_score,
        }
    }

    pub fn get_winner(&self) -> Option<Side> {
        self.winner
    }

    /// The side that just scored, while the next serve is still pending.
    pub fn get_point_scorer(&self) -> Option<Side> {
        self.serve_delay.as_ref().map(|delay| delay.scored_by)
    }

    /// What one paddle's controller sees. The left paddle's view is mirrored,
    /// so every controller sees itself defending the right edge.
    pub fn observe(&self, side: Side) -> Observation {
        let mut ball_position = self.ball.get_position();
        let mut ball_velocity = self.ball.get_velocity();
        if side == Side::Left {
            ball_position.x = COURT_WIDTH - ball_position.x;
            ball_velocity.x = -ball_velocity.x;
        }

        Observation {
            ball_position,
            ball_velocity,
            paddle_y: self.get_paddle(side).get_position().y,
            opponent_y: self.get_paddle(side.other()).get_position().y,
        }
    }

    // Mutators
    /// Starts a new match: scores cleared, paddles centered, ball served to a random side.
    pub fn reset(&mut self) {
        self.left_score = 0;
        self.right_score = 0;
        self.serve_delay = None;
        self.winner = None;
        self.left.set_start_position();
        self.right.set_start_position();

        let receiver = if self.rng.gen_bool(0.5) { Side::Left } else { Side::Right };
        self.ball.serve(receiver, &mut self.rng);
    }

    /// Advances the match by one step of `DT` seconds. Does nothing once the match has a winner.
    pub fn step(&mut self, left_move: Direction, right_move: Direction) -> StepEvents {
        let mut events = StepEvents::default();
        if self.winner.is_some() {
            return events;
        }

        self.left.update_position(left_move);
        self.right.update_position(right_move);

        // Serve once the point flash has finished.
        if let Some(delay) = &mut self.serve_delay {
            delay.steps_left -= 1;
            if delay.steps_left == 0 {
                let receiver = delay.scored_by.other();
                self.serve_delay = None;
                self.ball.serve(receiver, &mut self.rng);
            }
        }

        self.ball.update();

        // While waiting to serve, the ball just flies off screen.
        if self.serve_delay.is_some() {
            return events;
        }

        self.ball.check_wall_collision();
        if self.ball.check_paddle_collision(&self.left) {
            events.paddle_hit = Some(Side::Left);
        }
        if self.ball.check_paddle_collision(&self.right) {
            events.paddle_hit = Some(Side::Right);
        }

        let ball_x = self.ball.get_position().x;
        if ball_x < 0.0 {
            self.award_point(Side::Right);
            events.point = Some(Side::Right);
        } else if ball_x > COURT_WIDTH {
            self.award_point(Side::Left);
            events.point = Some(Side::Left);
        }

        events
    }

    fn award_point(&mut self, scorer: Side) {
        let score = match scorer {
            Side::Left => &mut self.left_score,
            Side::Right => &mut self.right_score,
        };
        *score += 1;

        if *score >= WINNING_SCORE {
            self.winner = Some(scorer);
        } else {
            self.serve_delay = Some(ServeDelay { scored_by: scorer, steps_left: SERVE_DELAY_STEPS });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::Brain;
    use crate::controller::{Bot, Controller, REACTION_TIME};

    /// Plays one full match between two untrained bots, headless.
    /// Returns the final score and how many steps it took.
    fn play_match(seed: u64) -> (u32, u32, u32) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut left = Bot::new(Brain::random(&mut rng), REACTION_TIME);
        let mut right = Bot::new(Brain::random(&mut rng), REACTION_TIME);
        let mut sim = Sim::new(seed);
        let mut steps = 0;

        while sim.get_winner().is_none() {
            let left_move = left.act(&sim.observe(Side::Left));
            let right_move = right.act(&sim.observe(Side::Right));
            sim.step(left_move, right_move);
            steps += 1;
            assert!(steps < 1_000_000, "match never finished");
        }

        (sim.get_score(Side::Left), sim.get_score(Side::Right), steps)
    }

    #[test]
    fn match_plays_to_a_winner_without_a_window() {
        let (left, right, _) = play_match(1);
        assert!(left == WINNING_SCORE || right == WINNING_SCORE);
    }

    #[test]
    fn same_seed_replays_the_same_match() {
        assert_eq!(play_match(7), play_match(7));
    }
}
