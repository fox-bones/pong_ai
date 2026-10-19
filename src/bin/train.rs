//! Trains the bot by evolution and self-play.
//!
//! A population of random brains plays matches against the best brains so far.
//! Each generation, the brains that win the most points survive and breed
//! slightly mutated copies of themselves; the rest are discarded.
//!
//! Run with: cargo run --release --bin train [generations] [seed] [starting brain]
//!
//! Without a starting brain, training begins from random brains. Given a saved brain
//! (such as resources/brains/champion.txt), it begins from that brain and mutated copies of it,
//! to keep improving it.
//!
//! Saves the final champion to resources/brains/champion.txt, which the game loads.

use pong::brain::Brain;
use pong::controller::{Bot, Controller, REACTION_TIME};
use pong::sim::{STEPS_PER_SECOND, Side, Sim};
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};
use std::ops::RangeInclusive;
use std::time::Instant;
use std::{env, fs, thread};

const DEFAULT_GENERATIONS: usize = 300;
const DEFAULT_SEED: u64 = 1;

const POPULATION: usize = 100;
/// The best brains of each generation, carried into the next unchanged.
const ELITES: usize = 10;
/// The rest of the next generation are mutated copies of brains from this many of the best.
const PARENTS: usize = 30;
/// Chance that each weight of a child is nudged, and the typical size of a nudge.
const MUTATION_RATE: f64 = 0.2;
const MUTATION_SIZE: f32 = 0.1;

/// Opponents each brain plays per generation, once from each side.
const OPPONENTS: usize = 4;
/// Opponents are drawn from this many of the most recent champions.
const RECENT_CHAMPIONS: usize = 10;
/// Opponents react anywhere in this range (in seconds), so brains learn to beat quick and
/// slow players alike, not just copies of themselves.
const OPPONENT_REACTION_TIMES: RangeInclusive<f32> = 0.1..=0.4;
/// Cap on a training match, in case neither side can score.
const MAX_MATCH_STEPS: u32 = 180 * STEPS_PER_SECOND;

/// How many of the last champions play the final round-robin that picks the one to save.
const FINALISTS: usize = 30;

const CHAMPION_PATH: &str = "resources/brains/champion.txt";

/// Someone for the population to play this generation. Every brain faces the same
/// opponents in the same matches, so they're compared fairly.
struct Opponent {
    brain: Brain,
    reaction_time: f32,
    seed: u64,
}

/// How one brain did across one or more matches.
#[derive(Clone, Copy, Default)]
struct MatchStats {
    returns: u32,
    points_won: u32,
    points_lost: u32,
    steps: u32,
}

impl MatchStats {
    fn add(&mut self, other: MatchStats) {
        self.returns += other.returns;
        self.points_won += other.points_won;
        self.points_lost += other.points_lost;
        self.steps += other.steps;
    }

    /// What evolution selects for: winning points and not losing them.
    fn fitness(&self) -> i64 {
        self.points_won as i64 - self.points_lost as i64
    }

    /// Share of the balls coming at this brain that it sent back.
    fn return_rate(&self) -> f32 {
        self.returns as f32 / (self.returns + self.points_lost).max(1) as f32
    }

    fn seconds_per_point(&self) -> f32 {
        self.steps as f32 / STEPS_PER_SECOND as f32 / (self.points_won + self.points_lost).max(1) as f32
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let generations: usize = args
        .next()
        .map(|arg| arg.parse().expect("generations must be a whole number"))
        .unwrap_or(DEFAULT_GENERATIONS);
    let seed: u64 = args
        .next()
        .map(|arg| arg.parse().expect("seed must be a whole number"))
        .unwrap_or(DEFAULT_SEED);
    let start_path = args.next();

    let mut rng = StdRng::seed_from_u64(seed);
    let mut population: Vec<Brain> = match &start_path {
        Some(path) => {
            let start = load_brain(path);
            std::iter::once(start.clone())
                .chain((1..POPULATION).map(|_| mutate(&start, &mut rng)))
                .collect()
        }
        None => (0..POPULATION).map(|_| Brain::random(&mut rng)).collect(),
    };
    // The best brain of each generation so far.
    let mut champions: Vec<Brain> = Vec::new();

    let start = start_path.as_deref().unwrap_or("random brains");
    println!("Training {generations} generations of {POPULATION} brains (seed {seed}), starting from {start}");
    let training_started = Instant::now();

    for generation in 1..=generations {
        let started = Instant::now();
        let opponents = choose_opponents(&population, &champions, &mut rng);
        let stats = evaluate(&population, &opponents);

        let mut ranking: Vec<usize> = (0..POPULATION).collect();
        ranking.sort_by_key(|&i| -stats[i].fitness());
        let best = &stats[ranking[0]];
        champions.push(population[ranking[0]].clone());

        println!(
            "gen {generation:>4} | champion won {:>3} points, lost {:>3}, returned {:>3.0}%, {:>3.0}s per point | {:.1}s",
            best.points_won,
            best.points_lost,
            best.return_rate() * 100.0,
            best.seconds_per_point(),
            started.elapsed().as_secs_f32(),
        );

        let mut next: Vec<Brain> = ranking[..ELITES].iter().map(|&i| population[i].clone()).collect();
        while next.len() < POPULATION {
            let parent = &population[ranking[rng.gen_range(0..PARENTS)]];
            next.push(mutate(parent, &mut rng));
        }
        population = next;
    }

    let (generation, champion) = final_champion(&champions, &mut rng);
    fs::write(CHAMPION_PATH, champion.to_text()).expect("couldn't save the champion");
    println!(
        "Done in {:.0}s. The champion of generation {generation} won the final round-robin; saved it to {CHAMPION_PATH}.",
        training_started.elapsed().as_secs_f32(),
    );
}

/// Picks the brain to save: whichever of the last few champions does best in a round-robin
/// against the others. One generation's winner can be lucky; this favors a brain that beats
/// many styles. Returns its generation number along with it.
fn final_champion(champions: &[Brain], rng: &mut StdRng) -> (usize, Brain) {
    let first = champions.len().saturating_sub(FINALISTS);
    let finalists = &champions[first..];
    let opponents: Vec<Opponent> = finalists
        .iter()
        .map(|brain| Opponent {
            brain: brain.clone(),
            reaction_time: rng.gen_range(OPPONENT_REACTION_TIMES),
            seed: rng.next_u64(),
        })
        .collect();

    let stats = evaluate(finalists, &opponents);
    let best = (0..finalists.len())
        .max_by_key(|&i| stats[i].fitness())
        .expect("train at least one generation");
    (first + best + 1, finalists[best].clone())
}

/// This generation's opponents: the latest champion plus others from the last few, so new
/// brains must beat the current best without forgetting how to beat recent styles.
/// Before there are champions, random members of the population stand in.
fn choose_opponents(population: &[Brain], champions: &[Brain], rng: &mut StdRng) -> Vec<Opponent> {
    let recent = &champions[champions.len().saturating_sub(RECENT_CHAMPIONS)..];

    (0..OPPONENTS)
        .map(|i| {
            let brain = match champions.last() {
                None => &population[rng.gen_range(0..population.len())],
                Some(latest) if i == 0 => latest,
                Some(_) => &recent[rng.gen_range(0..recent.len())],
            };
            Opponent {
                brain: brain.clone(),
                reaction_time: rng.gen_range(OPPONENT_REACTION_TIMES),
                seed: rng.next_u64(),
            }
        })
        .collect()
}

/// Plays every brain against the opponents, spread across all CPU cores.
fn evaluate(brains: &[Brain], opponents: &[Opponent]) -> Vec<MatchStats> {
    let threads = thread::available_parallelism().map_or(1, |n| n.get());
    let chunk_size = brains.len().div_ceil(threads);

    thread::scope(|scope| {
        let workers: Vec<_> = brains
            .chunks(chunk_size)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|brain| play_all(brain, opponents))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers.into_iter().flat_map(|worker| worker.join().unwrap()).collect()
    })
}

/// Plays a brain against each opponent from both sides of the court.
fn play_all(brain: &Brain, opponents: &[Opponent]) -> MatchStats {
    let mut total = MatchStats::default();
    for opponent in opponents {
        for side in [Side::Left, Side::Right] {
            total.add(play_match(brain, side, opponent));
        }
    }
    total
}

/// Plays one headless match and reports how `brain`, on `side`, did.
fn play_match(brain: &Brain, side: Side, opponent: &Opponent) -> MatchStats {
    let mut sim = Sim::new(opponent.seed);
    let mut me = Bot::new(brain.clone(), REACTION_TIME);
    let mut them = Bot::new(opponent.brain.clone(), opponent.reaction_time);
    let mut stats = MatchStats::default();

    while sim.get_winner().is_none() && stats.steps < MAX_MATCH_STEPS {
        let my_move = me.act(&sim.observe(side));
        let their_move = them.act(&sim.observe(side.other()));
        let events = match side {
            Side::Left => sim.step(my_move, their_move),
            Side::Right => sim.step(their_move, my_move),
        };
        stats.steps += 1;

        if events.paddle_hit == Some(side) {
            stats.returns += 1;
        }
        match events.point {
            Some(scorer) if scorer == side => stats.points_won += 1,
            Some(_) => stats.points_lost += 1,
            None => {}
        }
    }
    stats
}

fn load_brain(path: &str) -> Brain {
    let text = fs::read_to_string(path).unwrap_or_else(|err| panic!("couldn't read {path}: {err}"));
    Brain::from_text(&text).unwrap_or_else(|err| panic!("{path}: {err}"))
}

/// A copy of `parent` with some weights nudged at random.
fn mutate(parent: &Brain, rng: &mut StdRng) -> Brain {
    let weights = parent
        .get_weights()
        .iter()
        .map(|&weight| {
            if rng.gen_bool(MUTATION_RATE) {
                weight + MUTATION_SIZE * gaussian(rng)
            } else {
                weight
            }
        })
        .collect();
    Brain::from_weights(weights)
}

/// A sample from the standard normal distribution (Box–Muller).
fn gaussian(rng: &mut StdRng) -> f32 {
    let u1: f32 = rng.gen_range(f32::EPSILON..1.0);
    let u2: f32 = rng.gen_range(0.0..1.0);
    (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos()
}
