use std::time::Duration;

use super::{Refresh, Retry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceOpts {
    pub retry: Retry,
    pub refresh: Refresh,
}

impl Default for ResourceOpts {
    fn default() -> Self {
        Self {
            retry: Retry::none(),
            refresh: Refresh::None,
        }
    }
}

impl ResourceOpts {
    pub const fn retry(mut self, retry: Retry) -> Self {
        self.retry = retry;
        self
    }

    pub const fn refresh_every(mut self, period: Duration) -> Self {
        self.refresh = Refresh::Every(period);
        self
    }
}
