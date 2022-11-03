mod bot;
mod config;

use crate::config::Config;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
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
        .filter(Some("tracing::span"), log::LevelFilter::Warn)
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or_default())
        .init();

    let args = Args::parse();

    let config_path = PathBuf::from(args.config.as_deref().unwrap_or(config::DEFAULT_PATH));
    let config = match Config::load(&config_path) {
        Ok(config) => config,
        Err(e) => {
            log::error!("{e}");
            std::process::exit(1);
        }
    };
    let config = Arc::new(RwLock::new(config));
    log::info!("Config loaded successfully: {:#?}", config);

    let _watcher = match spawn_config_watcher(config_path, config.clone()) {
        Ok(watcher) => watcher,
        Err(e) => {
            log::error!("{e}");
            std::process::exit(1);
        }
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(bot::run(config));
}

fn spawn_config_watcher(
    path: PathBuf,
    config: Arc<RwLock<Config>>,
) -> notify::Result<impl Watcher> {
    let mut watcher = {
        let path = path.clone();

        RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                let event = result.unwrap();

                if event.kind.is_modify() {
                    match Config::load(&path) {
                        Ok(updated_config) => {
                            *config.write() = updated_config;
                            log::info!("Config reloaded");
                        }
                        Err(e) => log::error!("Unable to load updated config: {e}"),
                    }
                }
            },
            notify::Config::default(),
        )
    }?;

    watcher.watch(&path, RecursiveMode::NonRecursive)?;

    Ok(watcher)
}
