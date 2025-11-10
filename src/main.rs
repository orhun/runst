mod zbusNotify;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let server = zbus_notify::Notifications::new();
    server.run().await?;

    Ok(())
}
