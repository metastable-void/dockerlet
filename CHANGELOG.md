# Changelog

All notable changes to this crate are documented in this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-05-14

### Changed
- Internal Cargo.toml audit: `default-features = false` set on
  direct dependencies with explicit feature lists for what the
  crate actually uses. No behaviour change. (D24)

## [0.1.0] - 2026-05-13

- Initial substantive release. Provides the `GenericImage`
  builder, `Container` handle, `WaitFor` readiness probe, and
  cross-process concurrency-limit semaphore for workspace
  integration tests.
- Talks to the local Docker daemon over the Unix socket; no TLS at
  the dockerlet layer. Bollard feature set is deliberately narrow:
  only `pipe`, with no `home`, no `ssl`, no `ssl_providerless`, and
  no `rustls-native-certs`.
- **Container leak fix.** Consumers commonly stash a `Container`
  inside a `static OnceCell<...>` for a warm-test-fixture pattern;
  Rust doesn't run `Drop` on statics at process exit, which would
  leak the container indefinitely. dockerlet now registers a
  `libc::atexit(3)` hook on first container start that sends a
  best-effort stop (2-second SIGTERM grace) to every spawned
  container at process termination. Combined with Docker's
  `auto_remove: true` flag — now set by default on every
  dockerlet-spawned container — the daemon reaps the stopped
  container automatically. `Container::drop` unregisters from
  the atexit registry first so explicit Drop and atexit don't
  duplicate work.
- `WaitFor::message_on_stderr` / `message_on_stdout` fail fast
  when the container has exited before the readiness needle is
  found. Previously, bollard's `logs`-follow stream would EOF
  on a dead container and dockerlet would reopen the stream
  until the startup deadline elapsed (~3 minutes of waste per
  failure). Now an `inspect_container` between stream reopens
  short-circuits to `Error::ReadinessFailed` with the exit code.

## [0.0.0]

Name reservation on crates.io. No functional content yet.
