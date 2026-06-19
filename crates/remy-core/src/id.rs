use std::sync::Arc;

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct Id(Arc<str>);

impl Id {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for Id {
    fn from(s: &'static str) -> Self {
        Id(Arc::from(s))
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id(Arc::from(s))
    }
}

impl From<u32> for Id {
    fn from(n: u32) -> Self {
        Id(Arc::from(n.to_string()))
    }
}

impl From<u64> for Id {
    fn from(n: u64) -> Self {
        Id(Arc::from(n.to_string()))
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}