use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bollard::models::{ContainerCreateBody, HostConfig, PortBinding, PortMap};
use bollard::query_parameters::{CreateContainerOptionsBuilder, CreateImageOptionsBuilder};
use futures_util::StreamExt;

use crate::client::{DockerClient, connect};
use crate::concurrency::ConcurrencySlot;
use crate::{Container, ContainerPort, Error, Result, WaitFor};

const DEFAULT_STARTUP_TIMEOUT: Duration = Duration::from_secs(60);

/// Builder for a generic Docker image used by a test.
#[derive(Debug, Clone)]
pub struct GenericImage {
    repo: String,
    tag: String,
    exposed_ports: Vec<ContainerPort>,
    waits: Vec<WaitFor>,
    env: Vec<(String, String)>,
    startup_timeout: Duration,
}

impl GenericImage {
    /// Creates a new generic image builder from a repository and tag.
    pub fn new(repo: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            repo: repo.into(),
            tag: tag.into(),
            exposed_ports: Vec::new(),
            waits: Vec::new(),
            env: Vec::new(),
            startup_timeout: DEFAULT_STARTUP_TIMEOUT,
        }
    }

    /// Adds a port to expose and publish on an ephemeral host port.
    pub fn with_exposed_port(mut self, port: ContainerPort) -> Self {
        self.exposed_ports.push(port);
        self
    }

    /// Adds a readiness probe.
    pub fn with_wait_for(mut self, wait: WaitFor) -> Self {
        self.waits.push(wait);
        self
    }

    /// Adds an environment variable for the container.
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Sets the startup timeout.
    pub fn with_startup_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }

    /// Pulls, creates, starts, and waits for the container.
    pub async fn start(self) -> Result<Container> {
        let slot = ConcurrencySlot::acquire().await?;
        let timeout = self.startup_timeout;
        tokio::time::timeout(timeout, self.start_with_slot(slot))
            .await
            .map_err(|_| Error::StartupTimeout(timeout))?
    }

    async fn start_with_slot(self, slot: ConcurrencySlot) -> Result<Container> {
        let docker = connect()?;
        let image = self.image_name();
        pull_image_if_needed(&docker, &self.repo, &self.tag, &image).await?;
        let container_id = create_container(&docker, &image, &self).await?;
        docker.start_container(&container_id, None).await?;
        // Register for process-exit cleanup; pair with Docker's
        // `auto_remove: true` flag set in `create_container` so the
        // daemon reaps the container automatically after stop.
        // Containers that drop explicitly via `Container::drop`
        // unregister themselves first.
        crate::atexit::register(docker.clone(), container_id.clone());

        let waits = if self.waits.is_empty() {
            vec![WaitFor::Running]
        } else {
            self.waits
        };
        let deadline = Instant::now()
            .checked_add(self.startup_timeout)
            .ok_or_else(|| Error::Internal("startup timeout overflow".to_string()))?;
        for wait in waits {
            wait.wait(&docker, &container_id, deadline).await?;
        }

        Ok(Container::new(docker, container_id, slot))
    }

    fn image_name(&self) -> String {
        format!("{}:{}", self.repo, self.tag)
    }
}

async fn pull_image_if_needed(
    docker: &DockerClient,
    repo: &str,
    tag: &str,
    image: &str,
) -> Result<()> {
    if docker.inspect_image(image).await.is_ok() {
        return Ok(());
    }

    let options = CreateImageOptionsBuilder::new()
        .from_image(repo)
        .tag(tag)
        .build();
    let mut stream = docker.create_image(Some(options), None, None);
    while let Some(item) = stream.next().await {
        item.map_err(Error::from)?;
    }
    Ok(())
}

async fn create_container(
    docker: &DockerClient,
    image: &str,
    generic: &GenericImage,
) -> Result<String> {
    let options = CreateContainerOptionsBuilder::new()
        .name(&container_name())
        .build();
    let config = ContainerCreateBody {
        image: Some(image.to_string()),
        env: env_vars(generic),
        exposed_ports: exposed_ports(generic),
        host_config: Some(HostConfig {
            port_bindings: port_bindings(generic),
            publish_all_ports: Some(true),
            // `auto_remove: true` pairs with `atexit::register` in
            // `start_with_slot`: when the atexit hook (or
            // explicit Drop) sends a stop, the Docker daemon
            // removes the container automatically — no separate
            // remove_container call needed from atexit.
            auto_remove: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };

    let response = docker.create_container(Some(options), config).await?;
    Ok(response.id)
}

fn container_name() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    };
    format!("dockerlet-{}-{nanos}-{count}", std::process::id())
}

fn env_vars(generic: &GenericImage) -> Option<Vec<String>> {
    if generic.env.is_empty() {
        return None;
    }
    Some(
        generic
            .env
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect(),
    )
}

fn exposed_ports(generic: &GenericImage) -> Option<Vec<String>> {
    if generic.exposed_ports.is_empty() {
        return None;
    }
    Some(
        generic
            .exposed_ports
            .iter()
            .map(|port| port.as_docker_key())
            .collect(),
    )
}

fn port_bindings(generic: &GenericImage) -> Option<PortMap> {
    if generic.exposed_ports.is_empty() {
        return None;
    }

    let mut bindings: PortMap = HashMap::new();
    for port in &generic.exposed_ports {
        bindings.insert(
            port.as_docker_key(),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                host_port: Some(String::new()),
            }]),
        );
    }
    Some(bindings)
}
