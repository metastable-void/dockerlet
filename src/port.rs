/// A container port exposed by a test image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContainerPort {
    port: u16,
    protocol: Protocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Protocol {
    Tcp,
}

impl ContainerPort {
    /// Creates a TCP container port.
    pub fn tcp(port: u16) -> Self {
        Self {
            port,
            protocol: Protocol::Tcp,
        }
    }

    pub(crate) fn as_docker_key(self) -> String {
        let protocol = match self.protocol {
            Protocol::Tcp => "tcp",
        };
        format!("{}/{protocol}", self.port)
    }
}

/// Extension trait for `3306.tcp()`-style port declarations.
pub trait IntoContainerPort {
    /// Converts a numeric port into a TCP container port.
    fn tcp(self) -> ContainerPort;
}

impl IntoContainerPort for u16 {
    fn tcp(self) -> ContainerPort {
        ContainerPort::tcp(self)
    }
}
