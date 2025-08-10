# Battleship

This crate provides a Battleship game engine and AI players. It now includes a simulation binary that allows running AI vs AI games deterministically.

## Simulation

Run the simulation by providing two RNG seeds. The first player uses the first seed and takes the first move; the second player uses the second seed.

```bash
cargo run --bin sim -- <seed1> <seed2>
```

The program outputs a JSON object describing the result. Example:

```json
{"player1":{"guesses":48,"status":"Lost"},"player2":{"guesses":48,"status":"Won"},"winner":"player2"}
```

`status` indicates the outcome for each player, `guesses` counts shots taken, and `winner` names the winning side.
