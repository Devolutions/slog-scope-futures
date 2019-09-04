use std::borrow::Borrow;

use futures::{Future, Poll};
use slog::Logger;

use super::SlogScope;

impl<L, F> Future for SlogScope<L, F>
where
    F: Future,
    L: Borrow<Logger>,
{
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let inner = &mut self.inner;
        let logger = &self.logger;

        slog_scope::scope(logger.borrow(), || inner.poll())
    }
}

/// Convenience trait for wrapping a `0.1 Future` in a slog scope via method chaining.
///
/// Automatically implemented for all `0.1 Future`s.
pub trait FutureExt: Future + Sized {
    /// Wrap `self` in a slog scope
    fn with_logger<L>(self, logger: L) -> SlogScope<L, Self>
    where
        L: Borrow<Logger>,
    {
        SlogScope::new(logger, self)
    }
}

impl<F> FutureExt for F where F: Future {}
