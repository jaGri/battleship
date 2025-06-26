pub struct Board { /* grid, ships, hits/misses */ }
pub struct Ship { /* length, coords, orientation */ }

#[derive(Debug, Clone)]
pub enum GuessResult { Hit, Miss, Sink }
#[derive(Debug, Clone)]
pub enum GameStatus { InProgress, Won, Lost }
pub struct SyncPayload { /* serialized state diff */ }