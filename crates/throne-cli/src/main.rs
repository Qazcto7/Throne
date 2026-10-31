use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use throne_api::ApiServer;
use throne_core::parser::{parse_subscription, parse_uri};
use throne_core::{Profile, SingBoxGenerator};
use throne_db::Database;
use throne_service::ThroneEngine;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "throne")]
#[command(about = "Throne - Cross-platform proxy utility & client (Rust)", long_about = None)]
#[command(version)]
struct Cli {
    /// Custom path to SQLite database
    #[arg(short, long, global = true)]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run headless daemon service with remote API
    Daemon {
        /// Port for the Throne Remote API (default: 9095)
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Show current status and active proxy
    Status,
    /// Connect to a proxy profile by ID or name
    Connect {
        /// Profile ID or name
        target: String,
    },
    /// Disconnect active proxy
    Disconnect,
    /// List all proxy profiles and groups
    List,
    /// Measure latency for a profile or all profiles
    Test {
        /// Optional profile ID
        profile_id: Option<i64>,
    },
    /// Import proxy nodes from a URI, subscription link, or base64
    Import {
        /// URI or subscription URL or file path
        source: String,
        /// Group ID (defaults to 1)
        #[arg(short, long, default_value = "1")]
        group_id: i64,
    },
    /// Manage subscriptions
    Sub {
        #[command(subcommand)]
        cmd: SubCommands,
    },
    /// Toggle TUN mode (on / off)
    Tun {
        mode: String,
    },
    /// Toggle System Proxy (on / off)
    Sysproxy {
        mode: String,
    },
    /// Print or regenerate API key for thronectl / web client
    Key {
        #[arg(short, long)]
        regenerate: bool,
    },
    /// Export sing-box JSON configuration for a profile
    ExportConfig {
        /// Profile ID (defaults to active profile)
        profile_id: Option<i64>,
    },
}

#[derive(Subcommand)]
enum SubCommands {
    /// Update subscription nodes from remote URL
    Update {
        /// Group ID (defaults to 1)
        #[arg(default_value = "1")]
        group_id: i64,
    },
}

fn get_db_path(custom: Option<PathBuf>) -> PathBuf {
    if let Some(p) = custom {
        return p;
    }
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("throne");
    std::fs::create_dir_all(&config_dir).ok();
    config_dir.join("throne.db")
}

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    let cli = Cli::parse();
    let db_path = get_db_path(cli.db);
    let db = Database::open(&db_path)
        .with_context(|| format!("Failed to open database at {:?}", db_path))?;

    match cli.command {
        Commands::Daemon { port } => {
            let mut settings = db.settings.load_settings()?;
            if let Some(p) = port {
                settings.remote_api_port = p;
                db.settings.save_settings(&settings)?;
            }

            // Ensure secret key exists
            let key_bytes = if settings.remote_api_secret.is_empty() {
                let random_key = throne_api::generate_random_bytes(32);
                let text = URL_SAFE_NO_PAD.encode(&random_key);
                settings.remote_api_secret = text;
                db.settings.save_settings(&settings)?;
                random_key
            } else {
                URL_SAFE_NO_PAD
                    .decode(&settings.remote_api_secret)
                    .unwrap_or_else(|_| throne_api::generate_random_bytes(32))
            };

            info!("Throne Daemon starting...");
            info!("API Key: {}", settings.remote_api_secret);
            info!("Inbound Mixed Port: {}", settings.inbound_port);

            let engine = std::sync::Arc::new(ThroneEngine::new(db));
            let server_addr = format!("0.0.0.0:{}", settings.remote_api_port);
            let server = ApiServer::new(key_bytes, Arc::clone(&engine));

            tokio::select! {
                res = server.run(&server_addr) => {
                    if let Err(e) = res {
                        tracing::error!("Server error: {e}");
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal. Stopping Throne daemon...");
                    let _ = engine.disconnect().await;
                    info!("Throne daemon stopped cleanly.");
                }
            }
        }
        Commands::Status => {
            let engine = ThroneEngine::new(db);
            let status = engine.status().await?;
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        Commands::Connect { target } => {
            let engine = ThroneEngine::new(db.clone());
            let profile_id = if let Ok(id) = target.parse::<i64>() {
                id
            } else {
                let all = db.profiles.list_all()?;
                all.iter()
                    .find(|p| p.name == target)
                    .map(|p| p.id)
                    .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", target))?
            };

            engine.connect(profile_id).await?;
            println!("Connected to profile {}", profile_id);
        }
        Commands::Disconnect => {
            let engine = ThroneEngine::new(db);
            engine.disconnect().await?;
            println!("Disconnected");
        }
        Commands::List => {
            let groups = db.groups.list_all()?;
            let profiles = db.profiles.list_all()?;

            println!("{:<5} {:<25} {:<12} {:<10} {:<30}", "ID", "NAME", "PROTOCOL", "LATENCY", "GROUP");
            println!("{:-<85}", "");

            for p in profiles {
                let group_name = groups
                    .iter()
                    .find(|g| g.id == p.gid)
                    .map(|g| g.name.as_str())
                    .unwrap_or("Unknown");

                let latency_str = if p.latency > 0 {
                    format!("{} ms", p.latency)
                } else {
                    "-".to_string()
                };

                println!(
                    "{:<5} {:<25} {:<12} {:<10} {:<30}",
                    p.id, p.name, p.profile_type, latency_str, group_name
                );
            }
        }
        Commands::Test { profile_id } => {
            let engine = ThroneEngine::new(db.clone());
            if let Some(pid) = profile_id {
                let lat = engine.test_profile(pid).await?;
                match lat {
                    Some(ms) => println!("Profile {}: {} ms", pid, ms),
                    None => println!("Profile {}: Timeout / Error", pid),
                }
            } else {
                let all = db.profiles.list_all()?;
                println!("Testing {} profiles...", all.len());
                for p in all {
                    let lat = engine.test_profile(p.id).await?;
                    match lat {
                        Some(ms) => println!("{:<25} ({}): {} ms", p.name, p.id, ms),
                        None => println!("{:<25} ({}): Timeout / Error", p.name, p.id),
                    }
                }
            }
        }
        Commands::Import { source, group_id } => {
            let nodes = if source.starts_with("http://") || source.starts_with("https://") {
                let client = reqwest::Client::builder()
                    .user_agent("Throne/1.4.0 (Rust)")
                    .build()?;
                let resp = client.get(&source).send().await?.text().await?;
                parse_subscription(&resp)?
            } else if source.contains("://") {
                let node = parse_uri(&source)?;
                vec![node]
            } else if std::path::Path::new(&source).exists() {
                let content = std::fs::read_to_string(&source)?;
                parse_subscription(&content)?
            } else {
                parse_subscription(&source)?
            };

            let count = nodes.len();
            for (name, config) in nodes {
                let outbound_json = serde_json::to_string(&config)?;
                let profile = Profile {
                    name,
                    gid: group_id,
                    profile_type: config.protocol_name().to_string(),
                    outbound_json,
                    ..Default::default()
                };
                db.profiles.insert(&profile)?;
            }
            println!("Successfully imported {} node(s)", count);
        }
        Commands::Sub { cmd } => match cmd {
            SubCommands::Update { group_id } => {
                let engine = ThroneEngine::new(db);
                let count = engine.sub_updater.update_group(group_id).await?;
                println!("Subscription updated: {} nodes imported", count);
            }
        },
        Commands::Tun { mode } => {
            let engine = ThroneEngine::new(db);
            let enable = mode.to_lowercase() == "on" || mode == "1" || mode == "true";
            engine.toggle_tun(enable).await?;
            println!("TUN mode: {}", if enable { "enabled" } else { "disabled" });
        }
        Commands::Sysproxy { mode } => {
            let engine = ThroneEngine::new(db);
            let enable = mode.to_lowercase() == "on" || mode == "1" || mode == "true";
            engine.toggle_system_proxy(enable).await?;
            println!("System proxy: {}", if enable { "enabled" } else { "disabled" });
        }
        Commands::Key { regenerate } => {
            let mut settings = db.settings.load_settings()?;
            if regenerate || settings.remote_api_secret.is_empty() {
                let random_key = throne_api::generate_random_bytes(32);
                let text = URL_SAFE_NO_PAD.encode(&random_key);
                settings.remote_api_secret = text;
                db.settings.save_settings(&settings)?;
                println!("New API key generated: {}", settings.remote_api_secret);
            } else {
                println!("Current API key: {}", settings.remote_api_secret);
            }
        }
        Commands::ExportConfig { profile_id } => {
            let settings = db.settings.load_settings()?;
            let pid = profile_id
                .or(settings.active_profile_id)
                .ok_or_else(|| anyhow::anyhow!("No active profile and no profile ID specified"))?;

            let profile = db
                .profiles
                .get(pid)?
                .ok_or_else(|| anyhow::anyhow!("Profile {} not found", pid))?;

            let route_profile = match settings.active_route_id {
                Some(id) => db.routes.get_profile(id)?,
                None => None,
            };

            let route_rules = if let Some(ref rp) = route_profile {
                db.routes.list_rules_for_profile(rp.id)?
            } else {
                Vec::new()
            };

            let generator = SingBoxGenerator::new(
                &settings,
                &profile,
                route_profile.as_ref(),
                &route_rules,
            );
            let json = generator.generate()?;
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }

    Ok(())
}
