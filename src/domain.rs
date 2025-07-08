pub struct Board { /* grid, ships, hits/misses */ }
pub struct Ship { /* length, coords, orientation */ }

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum GuessResult { Hit, Miss, Sink }

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum GameStatus { InProgress, Won, Lost }

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct SyncPayload; /* serialized state diff */
