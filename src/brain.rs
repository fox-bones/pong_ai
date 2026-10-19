use crate::controller::Observation;
use crate::paddle::Paddle;
use crate::sim::{COURT_HEIGHT, COURT_WIDTH};
use rand::Rng;

const INPUTS: usize = 5;
const HIDDEN: usize = 16;
const OUTPUTS: usize = 1;

/// Every weight and bias in the network.
pub const WEIGHT_COUNT: usize = HIDDEN * (INPUTS + 1) + OUTPUTS * (HIDDEN + 1);

/// A small neural network that decides where the paddle should be.
///
/// One hidden layer of tanh units feeds a single output: the height the paddle's center
/// should move to. The `Bot` playing the brain moves the paddle there. Training evolves the weights.
#[derive(Clone)]
pub struct Brain {
    weights: Vec<f32>,
}

impl Brain {
    /// A brain with random weights, as training starts from.
    pub fn random(rng: &mut impl Rng) -> Self {
        Brain {
            weights: (0..WEIGHT_COUNT).map(|_| rng.gen_range(-1.0..1.0)).collect(),
        }
    }

    /// Panics unless there are exactly `WEIGHT_COUNT` weights.
    pub fn from_weights(weights: Vec<f32>) -> Self {
        assert_eq!(weights.len(), WEIGHT_COUNT, "a brain needs exactly {WEIGHT_COUNT} weights");
        Brain { weights }
    }

    /// Reads a brain saved by `to_text`.
    pub fn from_text(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        let header = lines.next().unwrap_or_default();
        if header != file_header() {
            return Err(format!("expected header \"{}\", found \"{header}\"", file_header()));
        }

        let weights = lines
            .map(|line| line.trim().parse::<f32>().map_err(|err| format!("bad weight \"{line}\": {err}")))
            .collect::<Result<Vec<f32>, String>>()?;
        if weights.len() != WEIGHT_COUNT {
            return Err(format!("expected {WEIGHT_COUNT} weights, found {}", weights.len()));
        }
        Ok(Brain { weights })
    }

    // Accessors
    pub fn get_weights(&self) -> &[f32] {
        &self.weights
    }

    /// Saves the brain as text: a header line naming the network's shape, then one weight per line.
    pub fn to_text(&self) -> String {
        let mut text = file_header();
        for weight in &self.weights {
            text.push('\n');
            text.push_str(&weight.to_string());
        }
        text.push('\n');
        text
    }

    /// Runs the network on an observation and returns the height the paddle's center should move to.
    pub fn target_y(&self, observation: &Observation) -> f32 {
        let inputs = encode(observation);
        let (hidden_weights, output_weights) = self.weights.split_at(HIDDEN * (INPUTS + 1));

        let mut hidden = [0.0; HIDDEN];
        for (unit, row) in hidden.iter_mut().zip(hidden_weights.chunks_exact(INPUTS + 1)) {
            *unit = weighted_sum(row, &inputs).tanh();
        }

        // The output's -1..1 spans the court, top to bottom.
        (weighted_sum(output_weights, &hidden).tanh() + 1.0) / 2.0 * COURT_HEIGHT
    }
}

/// Identifies a brain file and the network shape it was saved from,
/// so a file from a differently shaped network is rejected.
fn file_header() -> String {
    format!("pong-brain {INPUTS} {HIDDEN} {OUTPUTS}")
}

/// `row` is one unit's input weights followed by its bias.
fn weighted_sum(row: &[f32], inputs: &[f32]) -> f32 {
    let (bias, weights) = row.split_last().unwrap();
    bias + weights.iter().zip(inputs).map(|(w, x)| w * x).sum::<f32>()
}

/// Scales an observation to roughly -1..1, the range the network works in.
///
/// The brain's own paddle is left out: where the paddle should be doesn't depend on where it
/// is, and feeding it back in lets the target chase the paddle, which makes it jitter.
fn encode(observation: &Observation) -> [f32; INPUTS] {
    [
        (observation.ball_position.x / COURT_WIDTH * 2.0 - 1.0).clamp(-2.0, 2.0),
        observation.ball_position.y / COURT_HEIGHT * 2.0 - 1.0,
        (observation.ball_velocity.x / 600.0).clamp(-2.0, 2.0),
        (observation.ball_velocity.y / 300.0).clamp(-2.0, 2.0),
        (observation.opponent_y + Paddle::HEIGHT / 2.0) / COURT_HEIGHT * 2.0 - 1.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn saved_brain_loads_back_identical() {
        let brain = Brain::random(&mut StdRng::seed_from_u64(3));
        let loaded = Brain::from_text(&brain.to_text()).unwrap();
        assert_eq!(brain.get_weights(), loaded.get_weights());
    }

    #[test]
    fn rejects_a_brain_of_the_wrong_shape() {
        let text = format!("{}\n0.5\n", file_header());
        assert!(Brain::from_text(&text).is_err());
    }
}
