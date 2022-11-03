mod bot;
mod config;

use crate::config::Config;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use parking_lot::RwLock;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the config file
    #[arg(short, long)]
    config: Option<String>,
}

fn main() {
    pretty_env_logger::formatted_timed_builder()
        .filter(None, log::LevelFilter::Info)
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or_default())
        .init();

    let args = Args::parse();

    let config_path = PathBuf::from(args.config.as_deref().unwrap_or(config::DEFAULT_PATH));
    let config = match Config::load(&config_path) {
        Ok(config) => config,
        Err(e) => {
            log::error!("{}", e);
            std::process::exit(1);
        }
    };
    let config = Arc::new(RwLock::new(config));
    log::info!("Config loaded successfully.");
    println!("{:#?}", config);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(bot::run(config));
}
