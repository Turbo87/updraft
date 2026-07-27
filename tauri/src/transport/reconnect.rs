use std::time::Duration;

const INITIAL_DELAY: Duration = Duration::from_millis(250);
const MAX_DELAY: Duration = Duration::from_secs(10);

pub struct ReconnectBackoff {
    next_delay: Duration,
}

impl Default for ReconnectBackoff {
    fn default() -> Self {
        Self {
            next_delay: INITIAL_DELAY,
        }
    }
}

impl ReconnectBackoff {
    pub fn after_attempt(&mut self, delivered_bytes: bool) -> Duration {
        if delivered_bytes {
            self.next_delay = INITIAL_DELAY;
        }

        let delay = self.next_delay;
        self.next_delay = (delay * 2).min(MAX_DELAY);
        delay
    }
}

#[cfg(test)]
mod tests {
    use super::ReconnectBackoff;
    use std::time::Duration;

    #[test]
    fn empty_attempts_double_the_delay_to_the_cap() {
        let mut backoff = ReconnectBackoff::default();

        let delays: Vec<_> = (0..8).map(|_| backoff.after_attempt(false)).collect();

        assert_eq!(
            delays,
            [
                Duration::from_millis(250),
                Duration::from_millis(500),
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(4),
                Duration::from_secs(8),
                Duration::from_secs(10),
                Duration::from_secs(10),
            ]
        );
    }

    #[test]
    fn byte_carrying_attempt_resets_the_following_empty_attempt() {
        let mut backoff = ReconnectBackoff::default();

        assert_eq!(backoff.after_attempt(false), Duration::from_millis(250));
        assert_eq!(backoff.after_attempt(true), Duration::from_millis(250));
        assert_eq!(backoff.after_attempt(false), Duration::from_millis(500));
    }
}
