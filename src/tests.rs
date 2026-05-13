use std::time::Duration;

use crate::concurrency::compute_concurrency_limit_for;
use crate::{IntoContainerPort, WaitFor};

#[test]
fn concurrency_limit_formula_matches_dispatch() {
    assert_eq!(compute_concurrency_limit_for(1), 1);
    assert_eq!(compute_concurrency_limit_for(4), 1);
    assert_eq!(compute_concurrency_limit_for(8), 2);
    assert_eq!(compute_concurrency_limit_for(16), 4);
    assert_eq!(compute_concurrency_limit_for(64), 4);
}

#[test]
fn into_container_port_supports_tcp_suffix() {
    assert_eq!(3306_u16.tcp().as_docker_key(), "3306/tcp");
}

#[test]
fn wait_for_constructors_preserve_messages() {
    assert_eq!(
        WaitFor::message_on_stderr("ready"),
        WaitFor::MessageOnStderr("ready".to_string())
    );
    assert_eq!(
        WaitFor::message_on_stdout("done"),
        WaitFor::MessageOnStdout("done".to_string())
    );
    assert_eq!(
        WaitFor::Duration(Duration::from_secs(1)),
        WaitFor::Duration(Duration::from_secs(1))
    );
}
