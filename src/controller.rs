use crate::brain::Brain;
use crate::paddle::{Direction, Paddle};
use crate::sim::{DT, STEPS_PER_SECOND};
use macroquad::math::Vec2;
use std::collections::VecDeque;

/// How late, in seconds, bots see the ball when training, on Hard, and in Watch, like a human's reaction time.
/// Without it, simply following the ball would never miss, so nobody could score against a bot.
pub const REACTION_TIME: f32 = 0.2;

/// How far, in pixels, a bot's paddle must be from where its brain wants it before the bot moves.
/// Once it moves, it keeps going at full speed until it gets there, like a player holding an arrow key.
/// Without it, a bot could alternate moving and stopping every step and glide at any fraction of
/// full speed, which a player can't.
pub const MIN_MOVE: f32 = 20.0;

/// A moving bot stops once its paddle's center is this close to its target.
/// One step's travel, so it can't overshoot.
const ARRIVED: f32 = Paddle::SPEED * DT;

/// What a controller sees each step, from its own paddle's point of view.
///
/// The view is mirrored for the left paddle so that every controller sees itself
/// defending the right edge: a ball with positive x velocity is coming toward it.
/// That way one controller can play either side.
#[derive(Clone, Copy, Debug)]
pub struct Observation {
    pub ball_position: Vec2,
    pub ball_velocity: Vec2,
    /// Top edge of this controller's paddle.
    pub paddle_y: f32,
    /// Top edge of the other paddle.
    pub opponent_y: f32,
}

/// Anything that can drive a paddle: a human or a trained bot.
pub trait Controller {
    /// Chooses this paddle's move for the next step.
    fn act(&mut self, observation: &Observation) -> Direction;

    /// Clears anything remembered from the last match. Called when a new match starts.
    fn reset(&mut self) {}
}

/// A trained brain playing a paddle.
///
/// It sees the ball and the other paddle `reaction_time` seconds late,
/// but always knows where its own paddle is. It moves at one speed, like a player:
/// it waits until its paddle is `MIN_MOVE` from where the brain wants it, then goes all the way there.
pub struct Bot {
    brain: Brain,
    reaction_steps: usize,
    /// The last few observations, oldest first.
    recent: VecDeque<Observation>,
    /// The way the paddle is moving, like the arrow key a player is holding.
    moving: Direction,
}

impl Bot {
    pub fn new(brain: Brain, reaction_time: f32) -> Self {
        let reaction_steps = (reaction_time * STEPS_PER_SECOND as f32).round() as usize;
        Bot {
            brain,
            reaction_steps,
            recent: VecDeque::with_capacity(reaction_steps + 1),
            moving: Direction::None,
        }
    }
}

impl Controller for Bot {
    fn act(&mut self, observation: &Observation) -> Direction {
        self.recent.push_back(*observation);
        if self.recent.len() > self.reaction_steps + 1 {
            self.recent.pop_front();
        }

        let mut seen = self.recent[0];
        seen.paddle_y = observation.paddle_y;
        // How far the target is below the paddle's center; negative when above.
        let gap = self.brain.target_y(&seen) - (observation.paddle_y + Paddle::HEIGHT / 2.0);

        self.moving = match self.moving {
            Direction::Down if gap > ARRIVED => Direction::Down,
            Direction::Up if gap < -ARRIVED => Direction::Up,
            _ if gap > MIN_MOVE => Direction::Down,
            _ if gap < -MIN_MOVE => Direction::Up,
            _ => Direction::None,
        };
        self.moving
    }

    fn reset(&mut self) {
        self.recent.clear();
        self.moving = Direction::None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::WEIGHT_COUNT;

    /// A bot that sees instantly and always wants its paddle in the middle of the court:
    /// with every weight zero, the network's output is 0, which is the middle.
    fn centering_bot() -> Bot {
        Bot::new(Brain::from_weights(vec![0.0; WEIGHT_COUNT]), 0.0)
    }

    fn observation(paddle_y: f32) -> Observation {
        Observation {
            ball_position: Vec2::ZERO,
            ball_velocity: Vec2::ZERO,
            paddle_y,
            opponent_y: Paddle::START_Y,
        }
    }

    #[test]
    fn bot_stays_put_when_nearly_on_target() {
        let mut bot = centering_bot();
        assert_eq!(bot.act(&observation(Paddle::START_Y - MIN_MOVE + 1.0)), Direction::None);
        assert_eq!(bot.act(&observation(Paddle::START_Y + MIN_MOVE - 1.0)), Direction::None);
    }

    #[test]
    fn bot_moves_all_the_way_to_its_target_without_stopping() {
        let mut bot = centering_bot();
        let mut paddle_y = Paddle::START_Y - 3.0 * MIN_MOVE;
        while bot.act(&observation(paddle_y)) == Direction::Down {
            paddle_y += Paddle::SPEED * DT;
        }
        assert!((paddle_y - Paddle::START_Y).abs() <= ARRIVED, "stopped {} px from its target", paddle_y - Paddle::START_Y);
    }
}
