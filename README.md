# dockerlet

Minimal Docker test-container helper. Thin wrapper over
[`bollard`](https://docs.rs/bollard) with a deliberately
narrow configuration: local Unix-socket Docker daemon only,
no TLS at the client side, no auth file reads, no
`rustls-native-certs` / `webpki-roots` / `ring` pulled
into the dev-dep tree. Aimed at integration-test fixtures
that spin up a transient container (database, message broker,
…) per test and don't need the broader surface area of the
[`testcontainers`](https://docs.rs/testcontainers) crate.

**Status: 0.0.0 placeholder.** The substantive 0.1.0 surface
lands via the Philharmonic workspace's D23 dispatch — see the
crate's [`CHANGELOG.md`](CHANGELOG.md) and the workspace's
`docs/codex-prompts/2026-05-13-NNNN-d23-*` archive for the
full design.

Part of the Philharmonic workspace: https://github.com/metastable-void/philharmonic-workspace

## Contributing

This crate is developed as a submodule of the Philharmonic
workspace. Workspace-wide development conventions — git workflow,
script wrappers, Rust code rules, versioning, terminology — live
in the workspace meta-repo at
[metastable-void/philharmonic-workspace](https://github.com/metastable-void/philharmonic-workspace),
authoritatively in its
[`CONTRIBUTING.md`](https://github.com/metastable-void/philharmonic-workspace/blob/main/CONTRIBUTING.md).

SPDX-License-Identifier: Apache-2.0 OR MPL-2.0
