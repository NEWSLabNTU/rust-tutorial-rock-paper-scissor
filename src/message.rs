use serde::{Deserialize, Serialize};

/// The message that is exchanged between the players.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Hello { name: String },
    Leave { name: String },
    Act(Action),
}

/// Defines the action made by the player.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum Action {
    Rock = 0,
    Paper = 1,
    Scissor = 2,
}
