//! Process-exit cleanup for containers that outlive their Rust
//! [`Container`] handle.
//!
//! `Container::drop` handles per-instance cleanup, but consumers
//! commonly stash a [`Container`] inside a
//! `static OnceCell<Container>` (or equivalent) as a
//! warm-test-fixture pattern: spin one container up per test
//! binary, share it across every `#[test]` in that binary, isolate
//! by unique database name. Rust does **not** run `Drop` on
//! statics at normal process exit — see the Rust reference's
//! `static` chapter; statics are designed to live for the entire
//! program lifetime, and their destructors are not called on
//! termination. Without this module, those warm containers leak
//! at every test-binary exit: `docker ps` keeps listing
//! `dockerlet-*` mysqld/postgres processes from prior runs until
//! the user manually `docker rm -f`s them, and over a few days of
//! repeated test runs they pin enough kernel AIO contexts /
//! memory / port bindings to start interfering with new runs.
//!
//! This module closes the gap by registering a
//! [`libc::atexit(3)`] hook on first container start. The hook
//! spawns a brief single-threaded tokio runtime, asks the Docker
//! daemon to stop each registered container (with a 2-second
//! SIGTERM grace), and relies on Docker's `auto_remove: true`
//! flag (set in [`image::create_container`]) to remove the
//! container post-stop.
//!
//! Containers that explicitly drop via [`Container::drop`]
//! unregister themselves first so the atexit hook doesn't
//! re-attempt the work.
//!
//! [`libc::atexit(3)`]: https://man7.org/linux/man-pages/man3/atexit.3.html
//! [`Container`]: crate::Container
//! [`image::create_container`]: crate::image::create_container

use std::sync::{Mutex, OnceLock};

use bollard::query_parameters::StopContainerOptions;

use crate::client::DockerClient;

static REGISTRY: OnceLock<Mutex<Vec<(DockerClient, String)>>> = OnceLock::new();
static ATEXIT_REGISTERED: OnceLock<()> = OnceLock::new();

fn registry() -> &'static Mutex<Vec<(DockerClient, String)>> {
    REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

/// Register a running container for stop-on-process-exit cleanup.
///
/// Called by [`crate::image::GenericImage::start`] immediately
/// after `docker.start_container` returns success. The first
/// call also installs the libc::atexit hook.
pub(crate) fn register(docker: DockerClient, container_id: String) {
    if let Ok(mut guard) = registry().lock() {
        guard.push((docker, container_id));
    }
    ATEXIT_REGISTERED.get_or_init(|| {
        // SAFETY: `libc::atexit` registers a C function pointer
        // that the C runtime invokes once at normal process exit.
        // `atexit_cleanup` below is `extern "C"`, does not unwind
        // (panics inside are swallowed by the cleanup block), and
        // uses only OS-thread-safe primitives — calling it from
        // the C-side atexit dispatcher is safe. The hook is
        // idempotent on the registry: a re-entrant call (shouldn't
        // happen in practice; atexit is invoked once) would see
        // an emptied vec and no-op.
        unsafe {
            libc::atexit(atexit_cleanup);
        }
    });
}

/// Unregister a container from atexit cleanup.
///
/// Called by `Container::drop` so the explicit per-instance
/// cleanup path doesn't double up with the atexit hook.
pub(crate) fn unregister(container_id: &str) {
    if let Ok(mut guard) = registry().lock() {
        guard.retain(|(_, id)| id != container_id);
    }
}

extern "C" fn atexit_cleanup() {
    // Drain the registry under the lock; the rest of cleanup
    // happens outside the lock so we don't block any other
    // atexit handler that might want to add work.
    let containers: Vec<(DockerClient, String)> = match registry().lock() {
        Ok(mut guard) => std::mem::take(&mut *guard),
        Err(_) => return,
    };
    if containers.is_empty() {
        return;
    }

    // Brief single-threaded runtime so we can drive the bollard
    // stop API. atexit handlers can't .await; block_on() does.
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return,
    };

    runtime.block_on(async move {
        for (docker, id) in containers {
            let opts = StopContainerOptions {
                // Give the container 2s to handle SIGTERM cleanly
                // before SIGKILL. MySQL / Postgres respond to
                // SIGTERM with a graceful shutdown; 2s is enough
                // to flush, not enough to stall process exit.
                t: Some(2),
                ..Default::default()
            };
            // best-effort: container may already be stopped, the
            // daemon may be unreachable, etc. — none of these
            // should panic atexit.
            let _ = docker.stop_container(&id, Some(opts)).await;
            // `auto_remove: true` on container creation (see
            // `image::create_container`) tells the Docker daemon
            // to remove the container automatically once it
            // stops, so we don't need an explicit
            // `remove_container` call here.
        }
    });
}
