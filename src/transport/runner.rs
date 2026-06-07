//! Runner helper for bridging app commands and transport endpoints.

use std::vec::Vec;

use crate::app::{AppCommand, AppEvent, BattleshipApp};
use crate::transport::TransportEndpoint;

/// Bridges [`AppCommand::Send`] values and inbound transport messages.
///
/// Host runners remain responsible for rendering, persistence, agent prompts,
/// and process lifecycle commands. This helper only owns the transport-facing
/// work needed to keep remote app sessions moving.
pub struct TransportCommandRunner<T> {
    endpoint: T,
    connected: bool,
}

impl<T> TransportCommandRunner<T>
where
    T: TransportEndpoint,
{
    /// Create a runner around an app-facing transport endpoint.
    pub fn new(endpoint: T) -> Self {
        let connected = endpoint.is_connected();
        Self {
            endpoint,
            connected,
        }
    }

    /// Borrow the wrapped endpoint.
    pub fn endpoint(&self) -> &T {
        &self.endpoint
    }

    /// Mutably borrow the wrapped endpoint.
    pub fn endpoint_mut(&mut self) -> &mut T {
        &mut self.endpoint
    }

    /// Return the wrapped endpoint.
    pub fn into_endpoint(self) -> T {
        self.endpoint
    }

    /// Execute one transport pass against the app and command queue.
    ///
    /// This removes outbound send commands from `commands`, sends them through
    /// the endpoint, polls all currently available inbound messages, appends the
    /// resulting app commands, and emits connection-change events when the
    /// endpoint's connectivity changes.
    pub fn pump<A, O>(
        &mut self,
        app: &mut BattleshipApp<A, O>,
        commands: &mut Vec<AppCommand>,
    ) -> Result<(), T::Error> {
        self.emit_connection_transition(app, commands);
        self.send_pending(commands)?;
        self.poll_inbound(app, commands)?;
        self.emit_connection_transition(app, commands);
        Ok(())
    }

    fn send_pending(&mut self, commands: &mut Vec<AppCommand>) -> Result<(), T::Error> {
        let mut retained = Vec::with_capacity(commands.len());
        for command in commands.drain(..) {
            match command {
                AppCommand::Send(msg) => self.endpoint.send(&msg)?,
                other => retained.push(other),
            }
        }
        *commands = retained;
        Ok(())
    }

    fn poll_inbound<A, O>(
        &mut self,
        app: &mut BattleshipApp<A, O>,
        commands: &mut Vec<AppCommand>,
    ) -> Result<(), T::Error> {
        while let Some(msg) = self.endpoint.poll()? {
            commands.extend(app.update(AppEvent::Transport(msg)));
        }
        Ok(())
    }

    fn emit_connection_transition<A, O>(
        &mut self,
        app: &mut BattleshipApp<A, O>,
        commands: &mut Vec<AppCommand>,
    ) {
        let connected = self.endpoint.is_connected();
        if connected == self.connected {
            return;
        }

        self.connected = connected;
        let event = if connected {
            AppEvent::TransportConnected
        } else {
            AppEvent::TransportDisconnected
        };
        commands.extend(app.update(event));
    }
}
