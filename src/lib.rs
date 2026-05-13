//! Minimal Docker test-container helper.
//!
//! `dockerlet` is a thin wrapper over [`bollard`] aimed at projects
//! whose integration tests need to spin up a single transient
//! container per test (typical pattern: a database container, a
//! message broker, etc.) and don't need the broader surface area
//! of the [`testcontainers`] crate.
//!
//! The library is designed around a deliberately narrow
//! configuration of bollard:
//!
//! - Talk to the local Docker daemon over its Unix socket only —
//!   no TCP, no TLS at the bollard layer. Registry-side TLS for
//!   image pulls is the daemon's responsibility, not ours.
//! - No `~/.docker/config.json` auth read (`home` feature off) —
//!   tests use public images only. Authenticated registries are
//!   out of scope for v1.
//! - No `rustls-native-certs`, no `webpki-roots`, no `ring`. The
//!   workspace policy that bans those crates in release-binary
//!   runtime trees applies here too (`dockerlet` is the dev-dep
//!   that motivated removing the last `rustls-native-certs`
//!   wrapper from the philharmonic-workspace `deny.toml`).
//!
//! Status: 0.0.0 placeholder. The substantive 0.1.0 surface
//! ships in [ROADMAP D23 round 01]; see the crate's README + the
//! workspace's `docs/codex-prompts/2026-05-13-NNNN-d23-...` archive
//! for the full design.
//!
//! [`bollard`]: https://docs.rs/bollard
//! [`testcontainers`]: https://docs.rs/testcontainers

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
