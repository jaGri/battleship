#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Board { /* grid, ships, hits/misses */ }
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Ship { /* length, coords, orientation */ }

#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum GuessResult {
    Hit,
    Miss,
    Sink,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum GameStatus {
    InProgress,
    Won,
    Lost,
}

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct SyncPayload; /* serialized state diff */
