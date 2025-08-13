use std::{fmt::Display, net::SocketAddr};

#[derive(Debug, Clone, Default)]
pub struct SocketState {
    bound_addr: Option<SocketAddr>,
    pinned_addr: Option<SocketAddr>,
}

impl Display for SocketState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(i) = self.bound_addr() {
            write!(f, "{i}")
        } else {
            write!(f, "Unbound")
        }
    }
}

#[allow(unused)]
impl SocketState {
    #[must_use]
    #[inline]
    pub fn is_unpinned(&self) -> bool {
        !self.is_pinned()
    }

    #[must_use]
    #[inline]
    pub fn is_pinned(&self) -> bool {
        self.pinned_addr().is_some()
    }

    #[must_use]
    #[inline]
    pub const fn is_unbound(&self) -> bool {
        !self.is_bound()
    }

    #[must_use]
    #[inline]
    pub const fn is_bound(&self) -> bool {
        self.bound_addr().is_some()
    }

    #[must_use]
    pub fn pinned_addr(&self) -> Option<SocketAddr> {
        match (self.pinned_addr, self.bound_addr) {
            (Some(i), Some(j)) => (i == j).then_some(i),
            _ => self.pinned_addr,
        }
    }

    #[must_use]
    #[inline]
    pub const fn bound_addr(&self) -> Option<SocketAddr> {
        self.bound_addr
    }

    #[inline]
    pub fn pin(&mut self, maybe_addr: Option<SocketAddr>) {
        self.pinned_addr = maybe_addr;
    }

    #[must_use]
    #[inline]
    pub fn with_pin(mut self, addr: Option<SocketAddr>) -> Self {
        self.pin(addr);
        self
    }

    #[inline]
    pub fn bind(&mut self, maybe_addr: Option<SocketAddr>) {
        self.bound_addr = maybe_addr;
    }

    #[must_use]
    #[inline]
    pub fn with_bind(mut self, addr: Option<SocketAddr>) -> Self {
        self.bind(addr);
        self
    }
}
