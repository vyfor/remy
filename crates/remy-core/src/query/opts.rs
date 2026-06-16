use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueryOpts {
    pub dedupe: bool,
    pub cache_for: Option<Duration>,
}

impl QueryOpts {
    pub const fn dedupe(mut self) -> Self {
        self.dedupe = true;
        self
    }

    pub const fn cache_for(mut self, ttl: Duration) -> Self {
        self.cache_for = Some(ttl);
        self
    }
}
