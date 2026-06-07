#[cfg(not(feature = "std"))]
fn main() {}

#[cfg(feature = "std")]
use battleship::{
    AgentAction, AgentPrompt, AgentPromptKind, AgentRequest, AiAgent, AiDifficulty, AppCommand,
    AppEvent, AppState, BattleshipApp, Board, CliInput, CliRenderer, GuessBoard, HumanAgent,
    InputSource, PlacementMode, PlayerAgent, PlayerSide, Renderer, ShipPlacement, UiEvent, SHIPS,
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
struct CliRuntime {
    renderer: CliRenderer,
    input: CliInput,
    local_rng: SmallRng,
    opponent_rng: SmallRng,
    difficulty: AiDifficulty,
    quiet: bool,
}

#[cfg(feature = "std")]
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let local_rng = seeded_rng(cli.seed);
    let opponent_rng = seeded_rng(cli.seed.map(|seed| seed.wrapping_add(1)));

    let local = HumanAgent::default();
    let opponent = AiAgent::new(cli.difficulty);
    let mut app = BattleshipApp::new_local_ai(local, opponent);
    app.ai_difficulty = cli.difficulty;
    let mut runtime = CliRuntime {
        renderer: CliRenderer::new(),
        input: CliInput::new(),
        local_rng,
        opponent_rng,
        difficulty: cli.difficulty,
        quiet: cli.quiet,
    };
    let mut commands = vec![AppCommand::Render];

    loop {
        let exit = handle_commands(&mut app, &mut runtime, &mut commands)?;
        if exit {
            break;
        }

        commands = match app.state {
            AppState::Title
            | AppState::MainMenu
            | AppState::SoloSetup
            | AppState::DifficultyMenu
            | AppState::GameOver => read_shell_event(&app)?
                .map(|event| app.update(AppEvent::Ui(event)))
                .unwrap_or_else(|| vec![AppCommand::Render]),
            AppState::Playing if app.pending_prompt.is_none() => app.update(AppEvent::Tick),
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
    runtime: &mut CliRuntime,
    commands: &mut Vec<AppCommand>,
) -> anyhow::Result<bool> {
    while let Some(command) = pop_front(commands) {
        match command {
            AppCommand::Render if !runtime.quiet => runtime.renderer.render(&app.view())?,
            AppCommand::Render => {}
            AppCommand::Save(_) | AppCommand::ClearSave | AppCommand::Send(_) => {}
            AppCommand::ConfigureDifficulty(next) => {
                runtime.difficulty = next;
                app.opponent_agent = AiAgent::new(runtime.difficulty);
            }
            AppCommand::Exit => return Ok(true),
            AppCommand::RequestAgent(prompt) => {
                let action = handle_agent_prompt(app, runtime, prompt)?;
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
    runtime: &mut CliRuntime,
    prompt: AgentPrompt,
) -> anyhow::Result<Option<AgentAction>> {
    match prompt.kind {
        AgentPromptKind::PlaceShips(mode) => {
            let action = match prompt.side {
                PlayerSide::Local => read_ship_placement(app, &mut runtime.local_rng, mode)?,
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
                        &mut runtime.opponent_rng,
                    )
                    .map_err(|err| anyhow::anyhow!(err))?,
            };
            Ok(Some(action))
        }
        AgentPromptKind::SelectTarget => {
            let action = match prompt.side {
                PlayerSide::Local => {
                    let target = read_target(&mut runtime.input)?;
                    app.local_agent.on_ui_event(UiEvent::Target(target));
                    let guess_board = GuessBoard::from_engine(&app.match_state.local_engine);
                    let remaining = app.match_state.local_engine.enemy_ship_lengths_remaining();
                    app.local_agent
                        .handle_request(
                            AgentRequest::SelectTarget {
                                guess_board: &guess_board,
                                remaining_ships: &remaining,
                            },
                            &mut runtime.local_rng,
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
                            &mut runtime.opponent_rng,
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
                        .handle_request(AgentRequest::Observe(event), &mut runtime.local_rng);
                }
                PlayerSide::Opponent => {
                    let _ = app
                        .opponent_agent
                        .handle_request(AgentRequest::Observe(event), &mut runtime.opponent_rng);
                }
            }
            Ok(None)
        }
    }
}

#[cfg(feature = "std")]
fn read_ship_placement(
    app: &mut BattleshipApp<HumanAgent, AiAgent>,
    local_rng: &mut SmallRng,
    mode: PlacementMode,
) -> anyhow::Result<AgentAction> {
    match mode {
        PlacementMode::Manual => read_manual_placement(app, local_rng),
        PlacementMode::Random | PlacementMode::Prompt => read_random_placement(app, local_rng),
    }
}

#[cfg(feature = "std")]
fn read_random_placement(
    app: &mut BattleshipApp<HumanAgent, AiAgent>,
    local_rng: &mut SmallRng,
) -> anyhow::Result<AgentAction> {
    loop {
        app.local_agent.on_ui_event(UiEvent::RandomPlacement);
        let action = app
            .local_agent
            .handle_request(
                AgentRequest::PlaceShips {
                    board: app.match_state.local_engine.board(),
                },
                local_rng,
            )
            .map_err(|err| anyhow::anyhow!(err))?;

        println!("\nRandom placement preview:");
        if let Some(board) = board_from_action(&action) {
            CliRenderer::render_board_preview(&board);
        }
        prompt_line("Accept this board? [enter accept, r reroll, m manual]: ")?;
        match read_line()?.as_str() {
            "r" | "R" => continue,
            "m" | "M" => return read_manual_placement(app, local_rng),
            _ => return Ok(action),
        }
    }
}

#[cfg(feature = "std")]
fn read_manual_placement(
    app: &mut BattleshipApp<HumanAgent, AiAgent>,
    local_rng: &mut SmallRng,
) -> anyhow::Result<AgentAction> {
    app.local_agent.on_ui_event(UiEvent::ClearPlacements);
    let mut preview = app.match_state.local_engine.board().clone();

    for (ship_index, ship) in SHIPS.iter().enumerate() {
        loop {
            prompt_line(&format!(
                "Place {} (length {}) as '<coord> <H|V>' or r for random: ",
                ship.name(),
                ship.length()
            ))?;
            let line = read_line()?;
            if matches!(line.as_str(), "r" | "R") {
                return read_random_placement(app, local_rng);
            }

            let Some(placement) = CliInput::parse_placement(&line, ship_index) else {
                println!("Use a coordinate and orientation, such as A1 H or J6 V.");
                continue;
            };

            match preview.place(
                placement.ship_index,
                placement.row,
                placement.col,
                placement.orientation,
            ) {
                Ok(()) => {
                    app.local_agent.on_ui_event(UiEvent::PlaceShip {
                        ship_index: placement.ship_index,
                        row: placement.row,
                        col: placement.col,
                        orientation: placement.orientation,
                    });
                    CliRenderer::render_board_preview(&preview);
                    break;
                }
                Err(err) => {
                    println!("Unable to place ship there: {:?}.", err);
                }
            }
        }
    }

    app.local_agent
        .handle_request(
            AgentRequest::PlaceShips {
                board: app.match_state.local_engine.board(),
            },
            local_rng,
        )
        .map_err(|err| anyhow::anyhow!(err))
}

#[cfg(feature = "std")]
fn board_from_action(action: &AgentAction) -> Option<Board> {
    let AgentAction::PlaceShips(placements) = action else {
        return None;
    };
    board_from_placements(placements)
}

#[cfg(feature = "std")]
fn board_from_placements(placements: &[ShipPlacement]) -> Option<Board> {
    let mut board = Board::new();
    for placement in placements {
        board
            .place(
                placement.ship_index,
                placement.row,
                placement.col,
                placement.orientation,
            )
            .ok()?;
    }
    Some(board)
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
        AppState::MainMenu | AppState::SoloSetup | AppState::DifficultyMenu => {
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
