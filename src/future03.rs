use std::{
    borrow::Borrow,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use slog::Logger;

use super::SlogScope;

impl<L, F> Future for SlogScope<L, F>
where
    F: Future,
    L: Borrow<Logger>,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safety: We're not moving any of this, the inner future, or the logger.
        let this = unsafe { self.get_unchecked_mut() };
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
        let logger = &this.logger;

        slog_scope::scope(logger.borrow(), || inner.poll(cx))
    }
}

/// Convenience trait for wrapping a `std::future` in a slog scope via method chaining.
///
/// Automatically implemented for all `std::future`s.
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
