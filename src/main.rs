#[cfg(not(feature = "std"))]
fn main() {}

#[cfg(feature = "std")]
use battleship::{
    AiAgent, AiDifficulty, AppCommand, AppState, BattleshipApp, CliRenderer, GameStatus, Renderer,
};
#[cfg(feature = "std")]
use clap::Parser;
#[cfg(feature = "std")]
use rand::rngs::SmallRng;
#[cfg(feature = "std")]
use rand::SeedableRng;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[cfg(feature = "std")]
struct Cli {
    #[arg(long, help = "Fix RNG seed for reproducible games")]
    seed: Option<u64>,
    #[arg(long, value_enum, default_value_t = AiDifficulty::Hard)]
    difficulty: AiDifficulty,
    #[arg(long, help = "Suppress per-turn rendering")]
    quiet: bool,
}

#[cfg(feature = "std")]
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut rng1 = seeded_rng(cli.seed);
    let mut rng2 = seeded_rng(cli.seed.map(|seed| seed.wrapping_add(1)));

    let local = AiAgent::new(cli.difficulty);
    let opponent = AiAgent::new(cli.difficulty);
    let mut app = BattleshipApp::new_local_ai(local, opponent);
    let mut renderer = CliRenderer::new();

    let commands = app
        .place_ships(&mut rng1, &mut rng2)
        .map_err(|err| anyhow::anyhow!(err))?;
    handle_commands(&mut renderer, &app, commands, cli.quiet)?;

    while app.state == AppState::Playing {
        let commands = app
            .play_next_turn(&mut rng1, &mut rng2)
            .map_err(|err| anyhow::anyhow!(err))?;
        handle_commands(&mut renderer, &app, commands, cli.quiet)?;
    }

    match app.match_state.local_engine.status() {
        GameStatus::Won => println!("Local AI won."),
        GameStatus::Lost => println!("Opponent AI won."),
        GameStatus::InProgress => println!("Game ended before completion."),
    }

    Ok(())
}

#[cfg(feature = "std")]
fn seeded_rng(seed: Option<u64>) -> SmallRng {
    if let Some(seed) = seed {
        SmallRng::seed_from_u64(seed)
    } else {
        let mut seed_rng = rand::rng();
        SmallRng::from_rng(&mut seed_rng)
    }
}

#[cfg(feature = "std")]
fn handle_commands<A, O>(
    renderer: &mut CliRenderer,
    app: &BattleshipApp<A, O>,
    commands: Vec<AppCommand>,
    quiet: bool,
) -> anyhow::Result<()> {
    for command in commands {
        match command {
            AppCommand::Render if !quiet => renderer.render(&app.view())?,
            AppCommand::Render => {}
            AppCommand::Save(_) | AppCommand::ClearSave => {}
            AppCommand::Send(_) | AppCommand::RequestAgent(_) => {}
        }
    }
    Ok(())
}
