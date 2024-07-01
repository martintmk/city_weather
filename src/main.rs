use weather::app;

#[tokio::main]
async fn main() {
    app::run().await;
}
