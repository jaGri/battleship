use battleship::protocol::Message;
use battleship::domain::{GuessResult, GameStatus, Ship, SyncPayload};
use battleship::{GameState, GuessBoardState, BoardState, BitBoard, ShipState};
use proptest::prelude::*;

/// Generate arbitrary messages for fuzzing
fn arb_message() -> impl Strategy<Value = Message> {
    prop_oneof![
        (any::<u8>()).prop_map(|v| Message::Handshake { version: v }),
        (any::<u8>()).prop_map(|v| Message::HandshakeAck { version: v }),
        (any::<u8>(), any::<u64>(), any::<u8>(), any::<u8>()).prop_map(|(v, s, x, y)| {
            Message::Guess {
                version: v,
                seq: s,
                x,
                y,
            }
        }),
        (any::<u8>(), any::<u64>()).prop_map(|(v, s)| Message::StatusReq {
            version: v,
            seq: s,
        }),
        (any::<u8>(), any::<u64>(), arb_guess_result()).prop_map(|(v, s, res)| {
            Message::StatusResp {
                version: v,
                seq: s,
                res,
            }
        }),
        (any::<u8>(), any::<u64>(), arb_sync_payload()).prop_map(|(v, s, payload)| {
            Message::Sync {
                version: v,
                seq: s,
                payload,
            }
        }),
        (any::<u8>(), any::<u64>(), any::<usize>()).prop_map(|(v, s, id)| {
            Message::ShipStatusReq {
                version: v,
                seq: s,
                id,
            }
        }),
        (any::<u8>(), any::<u64>(), arb_ship()).prop_map(|(v, s, ship)| {
            Message::ShipStatusResp {
                version: v,
                seq: s,
                ship,
            }
        }),
        (any::<u8>(), any::<u64>()).prop_map(|(v, s)| Message::GameStatusReq {
            version: v,
            seq: s,
        }),
        (any::<u8>(), any::<u64>(), arb_game_status()).prop_map(|(v, s, status)| {
            Message::GameStatusResp {
                version: v,
                seq: s,
                status,
            }
        }),
        (any::<u8>(), any::<u64>()).prop_map(|(v, s)| Message::Ack { version: v, seq: s }),
        (any::<u8>()).prop_map(|v| Message::Heartbeat { version: v }),
    ]
}

fn arb_guess_result() -> impl Strategy<Value = GuessResult> {
    prop_oneof![
        Just(GuessResult::Hit),
        Just(GuessResult::Miss),
        any::<String>().prop_map(|name| GuessResult::Sink(name)),
    ]
}

fn arb_game_status() -> impl Strategy<Value = GameStatus> {
    prop_oneof![
        Just(GameStatus::InProgress),
        Just(GameStatus::Won),
        Just(GameStatus::Lost),
    ]
}

fn arb_ship() -> impl Strategy<Value = Ship> {
    (
        any::<String>(),
        any::<bool>(),
        prop_oneof![
            Just(None),
            (any::<u8>(), any::<u8>(), prop_oneof![Just(battleship::Orientation::Horizontal), Just(battleship::Orientation::Vertical)])
                .prop_map(|(r, c, o)| Some((r, c, o)))
        ],
    )
        .prop_map(|(name, sunk, position)| Ship {
            name,
            sunk,
            position,
        })
}

fn arb_sync_payload() -> impl Strategy<Value = SyncPayload> {
    (arb_game_state(), any::<[bool; 5]>()).prop_map(|(game_state, enemy_ships_remaining)| {
        SyncPayload {
            game_state,
            enemy_ships_remaining,
        }
    })
}

fn arb_game_state() -> impl Strategy<Value = GameState> {
    (
        arb_board_state(),
        arb_guess_board_state(),
        any::<[bool; 5]>(),
        any::<usize>(),
    )
        .prop_map(|(my_board, my_guesses, enemy_ships_remaining, enemy_remaining)| GameState {
            my_board,
            my_guesses,
            enemy_ships_remaining,
            enemy_remaining,
        })
}

fn arb_board_state() -> impl Strategy<Value = BoardState> {
    (
        prop::array::uniform5(arb_ship_state()),
        any::<u128>(),
        any::<u128>(),
        any::<u128>(),
    )
        .prop_map(|(ship_states, ship_map_bits, hits_bits, misses_bits)| {
            let ship_map = BitBoard::<u128, 10>::from_raw(ship_map_bits);
            let hits = BitBoard::<u128, 10>::from_raw(hits_bits);
            let misses = BitBoard::<u128, 10>::from_raw(misses_bits);
            BoardState {
                ship_states,
                ship_map,
                hits,
                misses,
            }
        })
}

fn arb_ship_state() -> impl Strategy<Value = ShipState> {
    use battleship::SHIPS;
    (
        (0..SHIPS.len()).prop_map(|i| SHIPS[i].name()),
        any::<bool>(),
        prop_oneof![
            Just(None),
            (any::<usize>(), any::<usize>(), prop_oneof![Just(battleship::Orientation::Horizontal), Just(battleship::Orientation::Vertical)])
                .prop_map(|(r, c, o)| Some((r, c, o)))
        ],
    )
        .prop_map(|(name, sunk, position)| {
            let mut state = ShipState::new(name);
            state.sunk = sunk;
            state.position = position;
            state
        })
}

fn arb_guess_board_state() -> impl Strategy<Value = GuessBoardState> {
    (any::<u128>(), any::<u128>()).prop_map(|(hits_bits, misses_bits)| {
        let hits = BitBoard::<u128, 10>::from_raw(hits_bits);
        let misses = BitBoard::<u128, 10>::from_raw(misses_bits);
        GuessBoardState { hits, misses }
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// Fuzz test: any message should serialize and deserialize without panic
    #[test]
    fn fuzz_message_serialization(msg in arb_message()) {
        let serialized = bincode::serialize(&msg);
        prop_assert!(serialized.is_ok());
        
        if let Ok(bytes) = serialized {
            let deserialized: Result<Message, _> = bincode::deserialize(&bytes);
            // Should either succeed or fail gracefully
            match deserialized {
                Ok(_) => {
                    // Successfully round-tripped
                }
                Err(_) => {
                    // Failed to deserialize - acceptable for fuzz test
                }
            }
        }
    }

    /// Fuzz test: arbitrary byte sequences should not crash deserializer
    #[test]
    fn fuzz_arbitrary_bytes(bytes in prop::collection::vec(any::<u8>(), 0..1000)) {
        let result: Result<Message, _> = bincode::deserialize(&bytes);
        // Should not panic, just return Ok or Err
        let _ = result;
    }

    /// Fuzz test: verify sync payload serialization
    #[test]
    fn fuzz_sync_payload(payload in arb_sync_payload()) {
        let serialized = bincode::serialize(&payload);
        prop_assert!(serialized.is_ok());
        
        if let Ok(bytes) = serialized {
            let deserialized: Result<SyncPayload, _> = bincode::deserialize(&bytes);
            if let Ok(restored) = deserialized {
                // Basic sanity checks
                prop_assert_eq!(restored.enemy_ships_remaining.len(), 5);
            }
        }
    }

    /// Fuzz test: game state serialization
    #[test]
    fn fuzz_game_state(state in arb_game_state()) {
        let serialized = bincode::serialize(&state);
        prop_assert!(serialized.is_ok());
        
        if let Ok(bytes) = serialized {
            let deserialized: Result<GameState, _> = bincode::deserialize(&bytes);
            if let Ok(restored) = deserialized {
                // Verify array sizes
                prop_assert_eq!(restored.enemy_ships_remaining.len(), 5);
            }
        }
    }

    /// Fuzz test: large messages
    #[test]
    fn fuzz_large_message(
        version in any::<u8>(),
        seq in any::<u64>(),
        name in prop::collection::vec(any::<u8>(), 1000..10000)
    ) {
        let large_name = String::from_utf8_lossy(&name).to_string();
        let msg = Message::StatusResp {
            version,
            seq,
            res: GuessResult::Sink(large_name),
        };
        
        let serialized = bincode::serialize(&msg);
        prop_assert!(serialized.is_ok());
    }

    /// Fuzz test: edge case coordinates
    #[test]
    fn fuzz_extreme_coordinates(
        version in any::<u8>(),
        seq in any::<u64>(),
        x in any::<u8>(),
        y in any::<u8>()
    ) {
        let msg = Message::Guess {
            version,
            seq,
            x,
            y,
        };
        
        let serialized = bincode::serialize(&msg);
        prop_assert!(serialized.is_ok());
        
        if let Ok(bytes) = serialized {
            let deserialized: Result<Message, _> = bincode::deserialize(&bytes);
            prop_assert!(deserialized.is_ok());
        }
    }

    /// Fuzz test: sequence number overflow
    #[test]
    fn fuzz_seq_overflow(seq in any::<u64>()) {
        let msg = Message::Guess {
            version: 1,
            seq,
            x: 0,
            y: 0,
        };
        
        let serialized = bincode::serialize(&msg);
        prop_assert!(serialized.is_ok());
        
        if let Ok(bytes) = serialized {
            let deserialized: Result<Message, _> = bincode::deserialize(&bytes);
            if let Ok(Message::Guess { seq: restored_seq, .. }) = deserialized {
                prop_assert_eq!(seq, restored_seq);
            }
        }
    }

    /// Fuzz test: truncated frames
    #[test]
    fn fuzz_truncated_frames(
        msg in arb_message(),
        truncate_at in 0usize..100
    ) {
        if let Ok(bytes) = bincode::serialize(&msg) {
            let truncated = &bytes[..truncate_at.min(bytes.len())];
            let result: Result<Message, _> = bincode::deserialize(truncated);
            // Should fail gracefully, not panic
            let _ = result;
        }
    }

    /// Fuzz test: corrupted frames
    #[test]
    fn fuzz_corrupted_frames(
        msg in arb_message(),
        corrupt_idx in 0usize..100,
        corrupt_byte in any::<u8>()
    ) {
        if let Ok(mut bytes) = bincode::serialize(&msg) {
            if corrupt_idx < bytes.len() {
                bytes[corrupt_idx] = corrupt_byte;
                let result: Result<Message, _> = bincode::deserialize(&bytes);
                // Should fail gracefully, not panic
                let _ = result;
            }
        }
    }

    /// Fuzz test: BitBoard serialization
    #[test]
    fn fuzz_bitboard_serialization(bits in any::<u128>()) {
        let bb = BitBoard::<u128, 10>::from_raw(bits);
        let serialized = bincode::serialize(&bb);
        prop_assert!(serialized.is_ok());
        
        if let Ok(bytes) = serialized {
            let deserialized: Result<BitBoard<u128, 10>, _> = bincode::deserialize(&bytes);
            if let Ok(restored) = deserialized {
                prop_assert_eq!(bb, restored);
            }
        }
    }
}

#[test]
fn test_specific_malformed_patterns() {
    // Test specific byte patterns known to be problematic
    
    // All zeros
    let all_zeros = vec![0u8; 100];
    let result: Result<Message, _> = bincode::deserialize(&all_zeros);
    assert!(result.is_err() || result.is_ok()); // Should not panic
    
    // All 0xFF
    let all_ff = vec![0xFFu8; 100];
    let result: Result<Message, _> = bincode::deserialize(&all_ff);
    assert!(result.is_err() || result.is_ok());
    
    // Alternating pattern
    let alternating: Vec<u8> = (0..100).map(|i| if i % 2 == 0 { 0xAA } else { 0x55 }).collect();
    let result: Result<Message, _> = bincode::deserialize(&alternating);
    assert!(result.is_err() || result.is_ok());
    
    // Random-ish pattern
    let pattern = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];
    let result: Result<Message, _> = bincode::deserialize(&pattern);
    assert!(result.is_err() || result.is_ok());
}

#[test]
fn test_empty_frame() {
    let empty = vec![];
    let result: Result<Message, _> = bincode::deserialize(&empty);
    assert!(result.is_err());
}

#[test]
fn test_single_byte_frame() {
    for byte in 0..=255u8 {
        let single = vec![byte];
        let result: Result<Message, _> = bincode::deserialize(&single);
        // Should not panic
        let _ = result;
    }
}
