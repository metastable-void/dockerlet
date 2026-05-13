# Changelog

All notable changes to this crate are documented in this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-05-13

- Initial substantive release. Provides the `GenericImage`
  builder, `Container` handle, `WaitFor` readiness probe, and
  cross-process concurrency-limit semaphore for workspace
  integration tests.
- Talks to the local Docker daemon over the Unix socket; no TLS at
  the dockerlet layer. Bollard feature set is deliberately narrow:
  only `pipe`, with no `home`, no `ssl`, no `ssl_providerless`, and
  no `rustls-native-certs`.

## [0.0.0]

Name reservation on crates.io. No functional content yet.
