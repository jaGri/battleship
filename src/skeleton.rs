use crate::{engine_api::GameApi, protocol::Message, transport::Transport};
pub struct Skeleton<E: GameApi, T: Transport> { engine: E, transport: T }
impl<E: GameApi, T: Transport> Skeleton<E, T> {
    pub async fn run(&mut self) -> anyhow::Result<()> {
        while let Ok(msg) = self.transport.recv().await {
            let reply = match msg {
                Message::Guess { x, y } => {
                    let res = self.engine.make_guess(x, y).await?;
                    Message::StatusResp(res)
                }
                Message::StatusReq => Message::StatusResp(self.engine.status()),
                Message::Sync(payload) => { self.engine.sync_state(payload).await?; Message::Ack },
                _ => Message::Ack,
            };
            self.transport.send(reply).await?;
        }
        Ok(())
    }
}