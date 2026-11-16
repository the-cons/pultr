#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Some("--version" | "-V") = std::env::args().nth(1).as_deref() {
        println!("pultr {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    pultr_core::run().await
}
