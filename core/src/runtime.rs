use tokio::sync::{mpsc, oneshot};

use crate::{App, Effect, Input, StateMessage};

const REQUEST_CAPACITY: usize = 16;
const STATE_CAPACITY: usize = 16;

enum RuntimeRequest {
    Submit(Input),
    Subscribe(oneshot::Sender<StateStream>),
}

pub struct CoreRuntime;

impl CoreRuntime {
    pub fn spawn(mut app: App) -> CoreRuntimeHandle {
        let (requests, mut receiver) = mpsc::channel(REQUEST_CAPACITY);

        tokio::spawn(async move {
            let mut subscribers: Vec<mpsc::Sender<StateMessage>> = Vec::new();
            while let Some(request) = receiver.recv().await {
                match request {
                    RuntimeRequest::Submit(input) => {
                        let update = app.handle(input);
                        update.effects.into_iter().for_each(execute_effect);
                        if !update.changes.is_empty() {
                            let message = StateMessage::Changes(update.changes);
                            subscribers
                                .retain(|subscriber| subscriber.try_send(message.clone()).is_ok());
                        }
                    }
                    RuntimeRequest::Subscribe(response) => {
                        let (sender, receiver) = mpsc::channel(STATE_CAPACITY);
                        sender
                            .try_send(StateMessage::Snapshot(app.snapshot()))
                            .expect("new state stream has capacity");
                        subscribers.push(sender);
                        let _ = response.send(StateStream { receiver });
                    }
                }
            }
        });

        CoreRuntimeHandle { requests }
    }
}

fn execute_effect(effect: Effect) {
    match effect {}
}

#[derive(Clone)]
pub struct CoreRuntimeHandle {
    requests: mpsc::Sender<RuntimeRequest>,
}

impl CoreRuntimeHandle {
    pub async fn submit(&self, input: Input) -> Result<(), RuntimeClosed> {
        self.requests
            .send(RuntimeRequest::Submit(input))
            .await
            .map_err(|_| RuntimeClosed)
    }

    pub async fn subscribe(&self) -> Result<StateStream, RuntimeClosed> {
        let (response, receiver) = oneshot::channel();
        self.requests
            .send(RuntimeRequest::Subscribe(response))
            .await
            .map_err(|_| RuntimeClosed)?;
        receiver.await.map_err(|_| RuntimeClosed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeClosed;

pub struct StateStream {
    receiver: mpsc::Receiver<StateMessage>,
}

impl StateStream {
    pub async fn recv(&mut self) -> Option<StateMessage> {
        self.receiver.recv().await
    }

    pub fn is_closed(&self) -> bool {
        self.receiver.is_closed()
    }
}
