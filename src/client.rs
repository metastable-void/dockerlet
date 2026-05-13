use std::env;

use bollard::Docker;

use crate::{Error, Result};

pub(crate) type DockerClient = Docker;

pub(crate) fn connect() -> Result<DockerClient> {
    let host =
        env::var("DOCKER_HOST").unwrap_or_else(|_| "unix:///var/run/docker.sock".to_string());
    if !host.starts_with("unix://") {
        return Err(Error::DaemonUnavailable(format!(
            "unsupported DOCKER_HOST {host:?}; dockerlet supports unix:// only"
        )));
    }
    Docker::connect_with_unix_defaults().map_err(Error::from)
}
