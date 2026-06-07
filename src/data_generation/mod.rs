//! Data-generation adapter boundary for AI simulations.

use crate::agent::AiDifficulty;

/// Configuration for generating AI-vs-AI game data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataGenerationConfig {
    pub games: usize,
    pub player_one: AiDifficulty,
    pub player_two: AiDifficulty,
}

impl Default for DataGenerationConfig {
    fn default() -> Self {
        Self {
            games: 100,
            player_one: AiDifficulty::Hard,
            player_two: AiDifficulty::Hard,
        }
    }
}
