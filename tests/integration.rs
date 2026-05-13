use std::time::Duration;

use dockerlet::{GenericImage, WaitFor};

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires Docker daemon"]
async fn starts_public_image_and_cleans_up() -> dockerlet::Result<()> {
    let _container = GenericImage::new("hello-world", "latest")
        .with_wait_for(WaitFor::message_on_stdout("Hello from Docker!"))
        .with_startup_timeout(Duration::from_secs(60))
        .start()
        .await?;
    Ok(())
}
