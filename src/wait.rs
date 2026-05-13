use std::time::{Duration, Instant};

use bollard::container::LogOutput;
use bollard::query_parameters::LogsOptionsBuilder;
use futures_util::StreamExt;
use tokio::time::sleep;

use crate::client::DockerClient;
use crate::{Error, Result};

/// Readiness probe used after the container starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitFor {
    /// Wait for a substring to appear on the container's stderr stream.
    MessageOnStderr(String),
    /// Wait for a substring to appear on the container's stdout stream.
    MessageOnStdout(String),
    /// Wait for Docker to report the container as running.
    Running,
    /// Sleep for a fixed duration after the container starts.
    Duration(Duration),
}

impl WaitFor {
    /// Creates a stderr substring readiness probe.
    pub fn message_on_stderr(msg: impl Into<String>) -> Self {
        Self::MessageOnStderr(msg.into())
    }

    /// Creates a stdout substring readiness probe.
    pub fn message_on_stdout(msg: impl Into<String>) -> Self {
        Self::MessageOnStdout(msg.into())
    }

    pub(crate) async fn wait(
        &self,
        docker: &DockerClient,
        container_id: &str,
        deadline: Instant,
    ) -> Result<()> {
        match self {
            Self::MessageOnStderr(message) => {
                wait_for_log(docker, container_id, LogStream::Stderr, message, deadline).await
            }
            Self::MessageOnStdout(message) => {
                wait_for_log(docker, container_id, LogStream::Stdout, message, deadline).await
            }
            Self::Running => wait_until_running(docker, container_id, deadline).await,
            Self::Duration(duration) => {
                sleep_until_deadline(*duration, deadline).await?;
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum LogStream {
    Stdout,
    Stderr,
}

async fn wait_for_log(
    docker: &DockerClient,
    container_id: &str,
    stream: LogStream,
    needle: &str,
    deadline: Instant,
) -> Result<()> {
    // Outer retry loop: bollard's log stream can close before the
    // container has produced the readiness message (transient
    // Docker daemon stream behaviour, particularly on slow image
    // starts). Reopen and continue reading until the needle
    // appears or the deadline elapses. Each reopen uses `tail("all")`
    // so we don't miss earlier output, paired with `follow(true)`
    // so the stream tracks new lines.
    //
    // **Fail fast on container exit.** If the container has died
    // (init crash, AIO exhaustion, etc.), bollard's `logs`-with-
    // `follow(true)` returns the historical lines and immediately
    // EOFs. Without an exit check we'd retry forever until the
    // deadline. Before each reopen, inspect the container state;
    // if it's not running, return `ReadinessFailed` with the exit
    // code so the caller knows the container is dead and won't
    // produce more output.
    loop {
        ensure_time_left(deadline)?;
        let options = LogsOptionsBuilder::new()
            .follow(true)
            .stdout(matches!(stream, LogStream::Stdout))
            .stderr(matches!(stream, LogStream::Stderr))
            .tail("all")
            .build();
        let mut logs = docker.logs(container_id, Some(options));

        loop {
            ensure_time_left(deadline)?;
            let remaining = deadline.saturating_duration_since(Instant::now());
            let next = tokio::time::timeout(remaining, logs.next()).await;
            let Ok(slot) = next else {
                return Err(Error::StartupTimeout(remaining));
            };
            let Some(item) = slot else {
                // Stream ended before the needle. If the container
                // is no longer running, fail fast; otherwise loop.
                break;
            };
            let output = item.map_err(Error::from)?;
            if log_matches(output, stream, needle) {
                return Ok(());
            }
        }

        // Inspect the container; if it has exited, give up.
        match docker.inspect_container(container_id, None).await {
            Ok(inspect) => {
                let running = inspect
                    .state
                    .as_ref()
                    .and_then(|state| state.running)
                    .unwrap_or(false);
                if !running {
                    let exit_code = inspect.state.as_ref().and_then(|state| state.exit_code);
                    return Err(Error::ReadinessFailed(format!(
                        "container exited (exit_code = {exit_code:?}) before readiness probe found `{needle}`"
                    )));
                }
            }
            Err(error) => {
                return Err(Error::ReadinessFailed(format!(
                    "could not inspect container during readiness probe: {error}"
                )));
            }
        }

        sleep(Duration::from_millis(50)).await;
    }
}

fn log_matches(output: LogOutput, stream: LogStream, needle: &str) -> bool {
    match (output, stream) {
        (LogOutput::StdErr { message }, LogStream::Stderr)
        | (LogOutput::StdOut { message }, LogStream::Stdout) => {
            String::from_utf8_lossy(&message).contains(needle)
        }
        _ => false,
    }
}

async fn wait_until_running(
    docker: &DockerClient,
    container_id: &str,
    deadline: Instant,
) -> Result<()> {
    loop {
        ensure_time_left(deadline)?;
        let inspect = docker.inspect_container(container_id, None).await?;
        if inspect
            .state
            .as_ref()
            .and_then(|state| state.running)
            .unwrap_or(false)
        {
            return Ok(());
        }
        sleep_until_deadline(Duration::from_millis(50), deadline).await?;
    }
}

async fn sleep_until_deadline(duration: Duration, deadline: Instant) -> Result<()> {
    ensure_time_left(deadline)?;
    let remaining = deadline.saturating_duration_since(Instant::now());
    sleep(std::cmp::min(duration, remaining)).await;
    ensure_time_left(deadline)
}

fn ensure_time_left(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        return Err(Error::ReadinessFailed(
            "readiness probe deadline elapsed".to_string(),
        ));
    }
    Ok(())
}
