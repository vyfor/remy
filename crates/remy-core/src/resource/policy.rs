use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retry {
    max_retries: u32,
    initial_delay: Duration,
    max_delay: Duration,
    backoff: Backoff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backoff {
    Fixed,
    Exponential,
}

impl Retry {
    pub const fn none() -> Self {
        Self {
            max_retries: 0,
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            backoff: Backoff::Fixed,
        }
    }

    pub const fn fixed(max_retries: u32, delay: Duration) -> Self {
        Self {
            max_retries,
            initial_delay: delay,
            max_delay: delay,
            backoff: Backoff::Fixed,
        }
    }

    pub const fn exponential(max_retries: u32, initial_delay: Duration) -> Self {
        Self {
            max_retries,
            initial_delay,
            max_delay: Duration::from_secs(30),
            backoff: Backoff::Exponential,
        }
    }

    pub const fn exponential_max(
        max_retries: u32,
        initial_delay: Duration,
        max_delay: Duration,
    ) -> Self {
        Self {
            max_retries,
            initial_delay,
            max_delay,
            backoff: Backoff::Exponential,
        }
    }

    pub(super) fn delay_for_retry(self, retry_number: u32) -> Option<Duration> {
        if retry_number == 0 || retry_number > self.max_retries {
            return None;
        }

        let delay = match self.backoff {
            Backoff::Fixed => self.initial_delay,
            Backoff::Exponential => {
                let shift = retry_number.saturating_sub(1).min(31);
                self.initial_delay.saturating_mul(1_u32 << shift)
            }
        };
        Some(delay.min(self.max_delay))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refresh {
    None,
    Every(Duration),
}
