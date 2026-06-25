use super::{Id, command::Action};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

/// Draw scheduler with per-window coalescing.
#[derive(Clone, Debug, Default)]
pub struct DrawScheduler {
    pub(crate) next: HashSet<Id>,
    pub(crate) delayed: HashMap<Id, Instant>,
}

impl DrawScheduler {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn request(&mut self, action: &Action) {
        match action {
            Action::DrawNow(id) | Action::DrawNext(id) => {
                self.next.insert(*id);
                self.delayed.remove(id);
            }
            Action::DrawAt { id, time } => {
                self.delayed
                    .entry(*id)
                    .and_modify(|stored| {
                        if *time < *stored {
                            *stored = *time;
                        }
                    })
                    .or_insert(*time);
            }
            Action::Batch(actions) => {
                for action in actions {
                    self.request(action);
                }
            }
            Action::Wait | Action::CloseRequested(_) | Action::Exit => {}
        }
    }

    #[must_use]
    pub(crate) fn take_ready(&mut self, now: Instant) -> Vec<Id> {
        let mut ready: Vec<Id> = self.next.drain().collect();
        let delayed_ready: Vec<Id> = self
            .delayed
            .iter()
            .filter_map(|(id, time)| (*time <= now).then_some(*id))
            .collect();
        for id in delayed_ready {
            self.delayed.remove(&id);
            ready.push(id);
        }
        ready.sort();
        ready.dedup();
        ready
    }

    #[must_use]
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.delayed.values().copied().min()
    }
}
