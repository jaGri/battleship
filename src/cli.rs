use crate::protocol::GameApi;
use std::io::{self, Write};

pub async fn run_cli(api: Box<dyn GameApi>) -> anyhow::Result<()> {
    loop {
        print!("> "); io::stdout().flush()?;
        let mut buf = String::new(); io::stdin().read_line(&mut buf)?;
        // parse commands like "guess 3 5", "status", etc.
    }
}
