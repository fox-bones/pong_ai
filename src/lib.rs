//! Pong's game rules, separate from the window.
//!
//! Nothing in this library opens a window, reads input, draws, or plays sound,
//! so a match can run headless and far faster than real time: for training and
//! testing, as well as for the game itself in `main.rs`.

pub mod ball;
pub mod brain;
pub mod controller;
pub mod paddle;
pub mod sim;
