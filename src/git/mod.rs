//! Git integration layer.
//!
//! This module wraps the actual backend implementation (`git2_backend`)
//! and re-exports only the stable public API (`ensure_repo`).
//!
//! The idea is to hide internal implementation details (currently based on `git2` crate)
//! so that future backends or alternative implementations could be swapped in
//! without affecting the rest of the codebase.

mod git2_backend;

/// Ensure that a git repository exists locally and is up-to-date.
///
/// This is the only public API exported from the `git` module.
/// Other modules should use this instead of depending directly on `git2_backend`.
pub use git2_backend::ensure_repo;
