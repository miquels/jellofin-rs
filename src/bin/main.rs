use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "jellofin-server")]
#[command(about = "Jellyfin-compatible media server", long_about = None)]
struct Args {
    #[arg(short, long, default_value = "jellofin-server.yaml")]
    config: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "jellofin_rs=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    if let Err(e) = jellofin_rs::run(&args.config).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
