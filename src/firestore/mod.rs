pub mod client;
pub mod query;
pub mod reference;
pub mod serde;

/// This module isn't really supposed to be exposed, but we are lacking
/// `#[cfg(doctest)]`, and we can't make it private either since doctests are
/// full-blown integration tests.
///
/// Relevant rust-lang issue: <https://github.com/rust-lang/rust/issues/67295>
pub mod test_helpers;

pub use reference::collection;
