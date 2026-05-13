use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;

use tokio::time::sleep;

use crate::{Error, Result};

static CONCURRENCY_LIMIT: LazyLock<usize> = LazyLock::new(compute_concurrency_limit);

/// A held cross-process container-start slot.
#[derive(Debug)]
pub(crate) struct ConcurrencySlot {
    file: File,
    path: PathBuf,
}

impl ConcurrencySlot {
    pub(crate) async fn acquire() -> Result<Self> {
        let lock_dir = lock_dir();
        fs::create_dir_all(&lock_dir)?;

        loop {
            for index in 0..concurrency_limit() {
                let path = lock_dir.join(format!("slot-{index}"));
                let file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&path)?;
                match try_lock_exclusive(&file) {
                    Ok(true) => return Ok(Self { file, path }),
                    Ok(false) => {}
                    Err(error) => return Err(error),
                }
            }
            sleep(Duration::from_millis(50)).await;
        }
    }
}

impl Drop for ConcurrencySlot {
    fn drop(&mut self) {
        if let Err(error) = unlock(&self.file) {
            tracing::warn!(
                path = %self.path.display(),
                error = %error,
                "failed to unlock dockerlet concurrency slot"
            );
        }
    }
}

pub(crate) fn concurrency_limit() -> usize {
    *CONCURRENCY_LIMIT
}

pub(crate) fn compute_concurrency_limit_for(parallelism: usize) -> usize {
    (parallelism / 4).clamp(1, 4)
}

fn compute_concurrency_limit() -> usize {
    let parallelism = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    compute_concurrency_limit_for(parallelism)
}

fn lock_dir() -> PathBuf {
    PathBuf::from(format!("/tmp/dockerlet-locks-{}", current_uid()))
}

fn current_uid() -> libc::uid_t {
    // SAFETY: `geteuid` has no preconditions and cannot invalidate Rust
    // memory; it only returns the effective uid of the current process.
    unsafe { libc::geteuid() }
}

fn try_lock_exclusive(file: &File) -> Result<bool> {
    let rc = flock(file, libc::LOCK_EX | libc::LOCK_NB);
    if rc == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(code) if code == libc::EWOULDBLOCK || code == libc::EAGAIN => Ok(false),
        _ => Err(Error::Internal(error.to_string())),
    }
}

fn unlock(file: &File) -> Result<()> {
    let rc = flock(file, libc::LOCK_UN);
    if rc == 0 {
        Ok(())
    } else {
        Err(Error::Internal(std::io::Error::last_os_error().to_string()))
    }
}

fn flock(file: &File, operation: libc::c_int) -> libc::c_int {
    // SAFETY: `file.as_raw_fd()` is a valid open file descriptor for
    // the duration of the call, and `operation` is one of the documented
    // `flock(2)` operation constants used by this module.
    unsafe { libc::flock(file.as_raw_fd(), operation) }
}
