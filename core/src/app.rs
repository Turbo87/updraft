use crate::flight::Flight;
use crate::{Change, Input, Snapshot, Update};

#[derive(Default)]
pub struct App {
    flight: Flight,
}

impl App {
    pub fn handle(&mut self, input: Input) -> Update {
        let changes = match input {
            Input::Flight(input) => self
                .flight
                .handle(input)
                .into_iter()
                .map(Change::Flight)
                .collect(),
        };

        Update {
            changes,
            effects: Vec::new(),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            position: self.flight.position(),
        }
    }
}
