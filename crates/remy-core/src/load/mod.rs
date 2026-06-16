#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Load {
    Initial,
    Loading,
    Success,
    Refreshing,
    Error,
}

impl Load {
    pub const fn is_initial(self) -> bool {
        matches!(self, Self::Initial)
    }

    pub const fn is_loading(self) -> bool {
        matches!(self, Self::Loading)
    }

    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }

    pub const fn is_refreshing(self) -> bool {
        matches!(self, Self::Refreshing)
    }

    pub const fn is_error(self) -> bool {
        matches!(self, Self::Error)
    }

    pub const fn is_pending(self) -> bool {
        matches!(self, Self::Loading | Self::Refreshing)
    }

    pub const fn is_settled(self) -> bool {
        !self.is_pending()
    }
}
