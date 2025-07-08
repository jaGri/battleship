use crate::{protocol::GameApi, protocol::Message, transport::Transport};

pub struct Skeleton<E: GameApi, T: Transport> {
    engine: E,
    transport: T,
}
impl<E: GameApi, T: Transport> Skeleton<E, T> {
    pub fn new(engine: E, transport: T) -> Self {
        Self { engine, transport }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        while let Ok(msg) = self.transport.recv().await {
            let reply = match msg {
                Message::Guess { x, y } => {
                    let res = self.engine.make_guess(x, y).await?;
                    Message::StatusResp(res)
                }
                Message::StatusReq => Message::Ack,
                Message::Sync(payload) => { self.engine.sync_state(payload).await?; Message::Ack },
                _ => Message::Ack,
            };
            self.transport.send(reply).await?;
        }
        Ok(())
    }

    pub async fn send(&mut self, msg: Message) -> anyhow::Result<Message> {
        self.transport.send(msg).await?;
        self.transport.recv().await
    }
}