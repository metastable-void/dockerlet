//! Minimal Docker test-container helper.
//!
//! `dockerlet` is a thin wrapper over [`bollard`] for integration
//! tests that need transient local Docker containers and only a small
//! subset of the broader `testcontainers` API.
//!
//! The crate talks to the local Docker daemon over a Unix socket. It
//! deliberately enables only bollard's `pipe` feature, which provides
//! Unix-socket transport on POSIX hosts without `home`, `ssl`,
//! `ssl_providerless`, `rustls-native-certs`, or registry-side TLS at
//! the bollard layer.

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

mod atexit;
mod client;
mod concurrency;
mod container;
mod error;
mod image;
mod port;
#[cfg(test)]
mod tests;
mod wait;

pub use container::Container;
pub use error::{Error, Result};
pub use image::GenericImage;
pub use port::{ContainerPort, IntoContainerPort};
pub use wait::WaitFor;
