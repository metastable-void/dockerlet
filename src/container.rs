use std::thread;

use bollard::query_parameters::{RemoveContainerOptions, StopContainerOptions};

use crate::client::DockerClient;
use crate::concurrency::ConcurrencySlot;
use crate::{ContainerPort, Error, Result};

/// Handle to a running Docker container.
#[derive(Debug)]
pub struct Container {
    docker: DockerClient,
    id: String,
    slot: Option<ConcurrencySlot>,
}

impl Container {
    pub(crate) fn new(docker: DockerClient, id: String, slot: ConcurrencySlot) -> Self {
        Self {
            docker,
            id,
            slot: Some(slot),
        }
    }

    /// Returns the host address used to reach published ports.
    pub async fn get_host(&self) -> Result<String> {
        Ok("127.0.0.1".to_string())
    }

    /// Returns the host-side IPv4 port mapped to a container TCP port.
    pub async fn get_host_port_ipv4(&self, container_port: ContainerPort) -> Result<u16> {
        let inspect = self.docker.inspect_container(&self.id, None).await?;
        let ports = inspect
            .network_settings
            .and_then(|settings| settings.ports)
            .ok_or_else(|| Error::ReadinessFailed("container port map unavailable".to_string()))?;
        let key = container_port.as_docker_key();
        let bindings = ports
            .get(&key)
            .and_then(|value| value.as_ref())
            .ok_or_else(|| Error::ReadinessFailed(format!("container port {key} is not mapped")))?;
        let binding = bindings.first().ok_or_else(|| {
            Error::ReadinessFailed(format!("container port {key} has no host binding"))
        })?;
        let host_port = binding
            .host_port
            .as_ref()
            .ok_or_else(|| Error::ReadinessFailed(format!("host port for {key} is absent")))?;
        host_port.parse::<u16>().map_err(|error| {
            Error::ReadinessFailed(format!("invalid host port {host_port}: {error}"))
        })
    }
}

impl Drop for Container {
    fn drop(&mut self) {
        // Unregister from atexit cleanup — this Drop is doing the
        // work explicitly, so the atexit hook shouldn't duplicate
        // on this container.
        crate::atexit::unregister(&self.id);

        let docker = self.docker.clone();
        let id = self.id.clone();
        let slot = self.slot.take();
        let builder = thread::Builder::new().name("dockerlet-drop".to_string());
        match builder.spawn(move || {
            if let Err(error) = cleanup_container(docker, id) {
                tracing::warn!(error = %error, "failed to clean up dockerlet container");
            }
            drop(slot);
        }) {
            Ok(handle) => {
                if handle.join().is_err() {
                    tracing::warn!("dockerlet cleanup thread panicked");
                }
            }
            Err(error) => {
                tracing::warn!(error = %error, "failed to spawn dockerlet cleanup thread");
            }
        }
    }
}

fn cleanup_container(docker: DockerClient, id: String) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(Error::from)?;
    runtime.block_on(async move {
        let stop_options = StopContainerOptions {
            t: Some(5),
            ..Default::default()
        };
        if let Err(error) = docker.stop_container(&id, Some(stop_options)).await {
            tracing::warn!(container = %id, error = %error, "failed to stop dockerlet container");
        }
        let remove_options = RemoveContainerOptions {
            v: true,
            force: true,
            ..Default::default()
        };
        docker
            .remove_container(&id, Some(remove_options))
            .await
            .map_err(Error::from)
    })
}
