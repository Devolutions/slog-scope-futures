//! # Slog Scopes for the Async World
//!
//! This crate provides a mechanism to use slog scopes with `Future`s.
//!
//! ## The Problem
//!
//! With synchronous code, slog-scope works as expected. But what about when
//! dealing with `async`/`await`?
//!
//! This won't compile:
//!
//! ```compile_fail
//! slog_scope::scope(&logger, || {
//!     some_operation().await // Error: can't use await outside of an async fn/block
//! })
//! ```
//!
//! This compiles, but doesn't do what you actually want:
//!
//! ```no_run
//! # #![feature(async_await)]
//! # async fn some_operation() {}
//! # async {
//! # use slog::o;
//! let logger = slog_scope::logger().new(o!("name" => "sub logger"));
//!
//! let fut = slog_scope::scope(&logger, async || { // <- scope start
//!     some_operation().await
//! }); // <- scope end
//!
//! fut.await // Scope not active here while the future is actually running
//! # };
//! ```
//!
//! ## The Solution
//!
//! Rather than using a closure to represent a slog scope, the logger must
//! instead be tied to the future itself, and its `poll` method wrapped in
//! a scope. The `SlogScope` type provides a `Future` wrapper that does exactly
//! that.
//!
//! ### Usage
//!
//! Using the wrapper directly:
//!
//! ```rust,norun
//! # #![feature(async_await)]
//! # async fn some_operation() {}
//! # async {
//! # use slog::o;
//! use slog_scope_futures::SlogScope;
//!
//! let logger = slog_scope::logger().new(o!("name" => "sub logger"));
//!
//! SlogScope::new(logger, some_operation()).await
//! # };
//! ```
//!
//! Using the convenience trait:
//!
//! ```rust,norun
//! # #![feature(async_await)]
//! # async fn some_operation() {}
//! # async {
//! # use slog::o;
//! use slog_scope_futures::FutureExt;
//!
//! let logger = slog_scope::logger().new(o!("name" => "sub logger"));
//!
//! some_operation().with_logger(logger).await
//! # };
//! ```
//!
//! ### Borrowed vs Owned Loggers
//!
//! Often, you need a `Future` to be `'static` so that it can be spawned into
//! an executor. Other times, though, you can get away with borrowing the
//! logger. This way, it can be re-used without additional cloning of the
//! handle.
//!
//! Because the `SlogScope` wrapper takes any `L: Borrow<Logger>`, you can
//! create it with either an owned *or* a borrowed `Logger`.
//!
//! ```rust,norun
//! # #![feature(async_await)]
//! # async fn some_operation() {}
//! # async fn some_other_operation() {}
//! # async {
//! # use slog::o;
//! # use core::future::Future;
//! use slog_scope_futures::FutureExt;
//!
//! let logger = slog_scope::logger().new(o!("name" => "sub logger"));
//!
//! some_operation().with_logger(&logger).await; // <- borrowed logger
//! let fut = some_other_operation().with_logger(logger); // <- owned logger
//!
//! async fn assert_static<F: Future + 'static>(f: F) -> F::Output { f.await }
//!
//! assert_static(fut).await
//! # };
//! ```
//!

#![warn(missing_docs)]

use std::borrow::Borrow;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use slog::Logger;

/// A `Future` wrapped in a slog scope.
pub struct SlogScope<L, F> {
    logger: L,
    inner: F,
}

impl<L, F> SlogScope<L, F>
where
    F: Future,
    L: Borrow<Logger>,
{
    /// Wrap a `Future` in a slog scope.
    pub fn new(logger: L, inner: F) -> Self {
        SlogScope { logger, inner }
    }
}

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

/// Convenience trait for wrapping a `Future` in a slog scope via method chaining.
///
/// Automatically implemented for all `Future`s.
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
