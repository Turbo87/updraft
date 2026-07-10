use crate::flight::Flight;
use crate::protocol::{Change, Input, Snapshot, Update};
use crate::timers::{Timer, Timers};

/// The application state machine. [`App::handle`] is the only mutation
/// entry point; everything else observes.
#[derive(Default)]
pub struct App {
    flight: Flight,
    timers: Timers,
}

impl App {
    pub fn handle(&mut self, input: Input) -> Update {
        let mut changes = Vec::new();
        let mut effects = Vec::new();

        match input {
            Input::Flight(input) => {
                self.flight
                    .handle(input, &mut self.timers, &mut changes, &mut effects);
            }
            Input::Clock(now) => {
                for timer in self.timers.advance(now) {
                    match timer {
                        Timer::PositionStaleness => {
                            self.flight.position_became_stale(&mut changes);
                        }
                    }
                }
            }
            Input::Job(outcome) => {
                self.flight
                    .job_finished(outcome, &mut changes, &mut effects);
            }
        }

        Update {
            changes: changes.into_iter().map(Change::Flight).collect(),
            effects,
            next_deadline: self.timers.next_deadline(),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            position: self.flight.position(),
            track_distance: self.flight.track_distance(),
        }
    }
}
