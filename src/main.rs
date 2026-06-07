#[cfg(not(feature = "std"))]
fn main() {}

#[cfg(feature = "std")]
use battleship::{
    AgentAction, AgentPrompt, AgentPromptKind, AgentRequest, AiAgent, AiDifficulty, AppCommand,
    AppEvent, AppState, BattleshipApp, CliInput, CliRenderer, GuessBoard, HumanAgent, InputSource,
    PlayerAgent, PlayerSide, Renderer, UiEvent,
};
#[cfg(feature = "std")]
use clap::Parser;
#[cfg(feature = "std")]
use rand::rngs::SmallRng;
#[cfg(feature = "std")]
use rand::SeedableRng;
#[cfg(feature = "std")]
use std::io::{self, Write};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[cfg(feature = "std")]
struct Cli {
    #[arg(long, help = "Fix RNG seed for reproducible games")]
    seed: Option<u64>,
    #[arg(long, value_enum, default_value_t = AiDifficulty::Hard)]
    difficulty: AiDifficulty,
    #[arg(long, help = "Suppress screen rendering")]
    quiet: bool,
}

#[cfg(feature = "std")]
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut local_rng = seeded_rng(cli.seed);
    let mut opponent_rng = seeded_rng(cli.seed.map(|seed| seed.wrapping_add(1)));

    let local = HumanAgent::default();
    let opponent = AiAgent::new(cli.difficulty);
    let mut app = BattleshipApp::new_local_ai(local, opponent);
    let mut renderer = CliRenderer::new();
    let mut input = CliInput::new();
    let mut commands = vec![AppCommand::Render];

    loop {
        let exit = handle_commands(
            &mut app,
            &mut renderer,
            &mut input,
            &mut local_rng,
            &mut opponent_rng,
            &mut commands,
            cli.quiet,
        )?;
        if exit {
            break;
        }

        commands = match app.state {
            AppState::Title | AppState::MainMenu | AppState::GameOver => read_shell_event(&app)?
                .map(|event| app.update(AppEvent::Ui(event)))
                .unwrap_or_else(|| vec![AppCommand::Render]),
            AppState::SoloSetup | AppState::Playing if app.pending_prompt.is_none() => {
                app.update(AppEvent::Tick)
            }
            AppState::Pairing | AppState::ConnectionOverlay => {
                let event = read_shell_event(&app)?;
                event
                    .map(|event| app.update(AppEvent::Ui(event)))
                    .unwrap_or_else(|| vec![AppCommand::Render])
            }
            _ => vec![AppCommand::Render],
        };
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
fn handle_commands(
    app: &mut BattleshipApp<HumanAgent, AiAgent>,
    renderer: &mut CliRenderer,
    input: &mut CliInput,
    local_rng: &mut SmallRng,
    opponent_rng: &mut SmallRng,
    commands: &mut Vec<AppCommand>,
    quiet: bool,
) -> anyhow::Result<bool> {
    while let Some(command) = pop_front(commands) {
        match command {
            AppCommand::Render if !quiet => renderer.render(&app.view())?,
            AppCommand::Render => {}
            AppCommand::Save(_) | AppCommand::ClearSave | AppCommand::Send(_) => {}
            AppCommand::Exit => return Ok(true),
            AppCommand::RequestAgent(prompt) => {
                let action = handle_agent_prompt(app, input, local_rng, opponent_rng, prompt)?;
                if let Some(action) = action {
                    commands.extend(app.update(AppEvent::Agent {
                        side: prompt.side,
                        action,
                    }));
                }
            }
        }
    }
    Ok(false)
}

#[cfg(feature = "std")]
fn handle_agent_prompt(
    app: &mut BattleshipApp<HumanAgent, AiAgent>,
    input: &mut CliInput,
    local_rng: &mut SmallRng,
    opponent_rng: &mut SmallRng,
    prompt: AgentPrompt,
) -> anyhow::Result<Option<AgentAction>> {
    match prompt.kind {
        AgentPromptKind::PlaceShips => {
            let action = match prompt.side {
                PlayerSide::Local => app
                    .local_agent
                    .handle_request(
                        AgentRequest::PlaceShips {
                            board: app.match_state.local_engine.board(),
                        },
                        local_rng,
                    )
                    .map_err(|err| anyhow::anyhow!(err))?,
                PlayerSide::Opponent => app
                    .opponent_agent
                    .handle_request(
                        AgentRequest::PlaceShips {
                            board: app
                                .match_state
                                .opponent_engine
                                .as_ref()
                                .ok_or_else(|| anyhow::anyhow!("missing opponent board"))?
                                .board(),
                        },
                        opponent_rng,
                    )
                    .map_err(|err| anyhow::anyhow!(err))?,
            };
            Ok(Some(action))
        }
        AgentPromptKind::SelectTarget => {
            let action = match prompt.side {
                PlayerSide::Local => {
                    let target = read_target(input)?;
                    app.local_agent.on_ui_event(UiEvent::Target(target));
                    let guess_board = GuessBoard::from_engine(&app.match_state.local_engine);
                    let remaining = app.match_state.local_engine.enemy_ship_lengths_remaining();
                    app.local_agent
                        .handle_request(
                            AgentRequest::SelectTarget {
                                guess_board: &guess_board,
                                remaining_ships: &remaining,
                            },
                            local_rng,
                        )
                        .map_err(|err| anyhow::anyhow!(err))?
                }
                PlayerSide::Opponent => {
                    let opponent = app
                        .match_state
                        .opponent_engine
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("missing opponent board"))?;
                    let guess_board = GuessBoard::from_engine(opponent);
                    let remaining = opponent.enemy_ship_lengths_remaining();
                    app.opponent_agent
                        .handle_request(
                            AgentRequest::SelectTarget {
                                guess_board: &guess_board,
                                remaining_ships: &remaining,
                            },
                            opponent_rng,
                        )
                        .map_err(|err| anyhow::anyhow!(err))?
                }
            };
            Ok(Some(action))
        }
        AgentPromptKind::Observe(event) => {
            match prompt.side {
                PlayerSide::Local => {
                    let _ = app
                        .local_agent
                        .handle_request(AgentRequest::Observe(event), local_rng);
                }
                PlayerSide::Opponent => {
                    let _ = app
                        .opponent_agent
                        .handle_request(AgentRequest::Observe(event), opponent_rng);
                }
            }
            Ok(None)
        }
    }
}

#[cfg(feature = "std")]
fn read_target(input: &mut CliInput) -> anyhow::Result<(usize, usize)> {
    loop {
        match input.poll_input()? {
            Some(UiEvent::Target(coord)) => return Ok(coord),
            Some(_) | None => {
                println!("Please enter a coordinate such as A1 or J10.");
            }
        }
    }
}

#[cfg(feature = "std")]
fn read_shell_event(app: &BattleshipApp<HumanAgent, AiAgent>) -> anyhow::Result<Option<UiEvent>> {
    match app.state {
        AppState::Title => {
            prompt_line("Press Enter to start, or q to quit: ")?;
            let line = read_line()?;
            if line.eq_ignore_ascii_case("q") {
                Ok(Some(UiEvent::Back))
            } else {
                Ok(Some(UiEvent::Start))
            }
        }
        AppState::MainMenu => {
            prompt_line("Menu command [w/s/enter/q]: ")?;
            match read_line()?.as_str() {
                "w" | "W" => Ok(Some(UiEvent::Up)),
                "s" | "S" => Ok(Some(UiEvent::Down)),
                "q" | "Q" => Ok(Some(UiEvent::Back)),
                _ => Ok(Some(UiEvent::Confirm)),
            }
        }
        AppState::GameOver => {
            prompt_line("Press Enter for the menu, or q to quit: ")?;
            if read_line()?.eq_ignore_ascii_case("q") {
                Ok(Some(UiEvent::Back))
            } else {
                Ok(Some(UiEvent::Confirm))
            }
        }
        AppState::Pairing | AppState::ConnectionOverlay => {
            prompt_line("Connection command [enter retry, b back]: ")?;
            match read_line()?.as_str() {
                "b" | "B" => Ok(Some(UiEvent::Back)),
                _ => Ok(Some(UiEvent::Confirm)),
            }
        }
        _ => Ok(None),
    }
}

#[cfg(feature = "std")]
fn prompt_line(prompt: &str) -> io::Result<()> {
    print!("{}", prompt);
    io::stdout().flush()
}

#[cfg(feature = "std")]
fn read_line() -> io::Result<String> {
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

#[cfg(feature = "std")]
fn pop_front<T>(items: &mut Vec<T>) -> Option<T> {
    if items.is_empty() {
        None
    } else {
        Some(items.remove(0))
    }
}
