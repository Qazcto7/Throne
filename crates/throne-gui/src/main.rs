mod app;
mod controller;
mod state;
mod theme;

use app::ThroneApp;
use controller::spawn_controller;
use eframe::egui;
use std::path::PathBuf;
use throne_db::Database;
use tokio::sync::mpsc::unbounded_channel;

fn get_db_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let p = parent.join("config").join("throne.db");
        if p.exists() {
            return p;
        }
    }
    let local_cfg = PathBuf::from("config/throne.db");
    if local_cfg.exists() {
        return local_cfg;
    }
    if let Some(home) = dirs::home_dir() {
        let byedpi_db = home.join("byedpi_throne_linux/Throne/config/throne.db");
        if byedpi_db.exists() {
            return byedpi_db;
        }
    }
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("throne");
    std::fs::create_dir_all(&config_dir).ok();
    config_dir.join("throne.db")
}

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let db_path = get_db_path();
    let db = Database::open(&db_path).unwrap_or_else(|e| {
        eprintln!("Failed to open database at {}: {}. Falling back to in-memory.", db_path.display(), e);
        Database::open_in_memory().expect("in-memory database creation failed")
    });

    let (cmd_tx, cmd_rx) = unbounded_channel();
    let (evt_tx, evt_rx) = std::sync::mpsc::channel();

    spawn_controller(db, cmd_rx, evt_tx);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([750.0, 480.0])
            .with_title("Throne"),
        ..Default::default()
    };

    eframe::run_native(
        "Throne",
        native_options,
        Box::new(|cc| {
            theme::apply_throne_theme(&cc.egui_ctx);
            Ok(Box::new(ThroneApp::new(cmd_tx, evt_rx, cc)))
        }),
    )
}
