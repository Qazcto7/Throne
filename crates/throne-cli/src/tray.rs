use ksni::menu::*;
use ksni::{MenuItem, Tray, TrayMethods};
use std::sync::Arc;
use throne_db::Database;
use throne_service::ThroneEngine;
use tokio::runtime::Handle as TokioHandle;
use tracing::info;

pub struct ThroneTray {
    pub db: Arc<Database>,
    pub engine: Arc<ThroneEngine>,
    pub tokio_handle: TokioHandle,
    pub is_connected: bool,
    pub active_profile_name: String,
    pub active_profile_id: Option<i64>,
    pub tun_enabled: bool,
    pub byedpi_preset: String,
}

impl Tray for ThroneTray {
    fn id(&self) -> String {
        "throne".into()
    }

    fn title(&self) -> String {
        if self.is_connected {
            format!("Throne (Rust) - {}", self.active_profile_name)
        } else {
            "Throne (Rust) - Bağlantı Yok".into()
        }
    }

    fn icon_theme_path(&self) -> String {
        if let Some(home) = dirs::home_dir() {
            let p = home.join(".local/share/Throne");
            if p.exists() {
                return p.to_string_lossy().to_string();
            }
        }
        "/home/kazim/byedpi_throne_linux/Throne".into()
    }

    fn icon_name(&self) -> String {
        if self.is_connected {
            "throne".into()
        } else {
            "network-vpn-disconnected-symbolic".into()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items = Vec::new();

        // 1. Status header
        let status_label = if self.is_connected {
            format!("🟢 Bağlı: {}", self.active_profile_name)
        } else {
            "⚪ Bağlantı Yok".into()
        };
        items.push(
            StandardItem {
                label: status_label,
                enabled: false,
                ..Default::default()
            }
            .into(),
        );

        items.push(MenuItem::Separator);

        // 2. Connect / Disconnect Action
        if self.is_connected {
            items.push(
                StandardItem {
                    label: "🛑 Bağlantıyı Kes".into(),
                    activate: Box::new(|tray: &mut Self| {
                        let engine = Arc::clone(&tray.engine);
                        tray.tokio_handle.spawn(async move {
                            let _ = engine.disconnect().await;
                        });
                        tray.is_connected = false;
                        tray.active_profile_id = None;
                        tray.active_profile_name = "Bağlantı Yok".into();
                    }),
                    ..Default::default()
                }
                .into(),
            );
        } else {
            // List profiles from database
            if let Ok(profiles) = self.db.profiles.list_all() {
                for p in profiles {
                    let pid = p.id;
                    let pname = p.name.clone();
                    let display_name = format!("⚡ Bağlan: {}", p.name);
                    items.push(
                        StandardItem {
                            label: display_name,
                            activate: Box::new(move |tray: &mut Self| {
                                let engine = Arc::clone(&tray.engine);
                                let target_name = pname.clone();
                                tray.tokio_handle.spawn(async move {
                                    let _ = engine.connect(pid).await;
                                });
                                tray.is_connected = true;
                                tray.active_profile_id = Some(pid);
                                tray.active_profile_name = target_name;
                            }),
                            ..Default::default()
                        }
                        .into(),
                    );
                }
            }
        }

        items.push(MenuItem::Separator);

        // 3. ByeDPI Options
        let is_vf = self.byedpi_preset == "vodafone";
        items.push(
            CheckmarkItem {
                label: "🛡️ ByeDPI (Vodafone / DPI Bypass)".into(),
                checked: is_vf,
                activate: Box::new(|tray: &mut Self| {
                    let mut settings = tray.db.settings.load_settings().unwrap_or_default();
                    if tray.byedpi_preset == "vodafone" {
                        tray.byedpi_preset = "standard".into();
                        settings.byedpi_preset = "standard".into();
                    } else {
                        tray.byedpi_preset = "vodafone".into();
                        settings.byedpi_preset = "vodafone".into();
                    }
                    let _ = tray.db.settings.save_settings(&settings);
                }),
                ..Default::default()
            }
            .into(),
        );

        // 4. TUN Mode Toggle
        items.push(
            CheckmarkItem {
                label: "🌐 TUN Modu (Tüm Sistem)".into(),
                checked: self.tun_enabled,
                activate: Box::new(|tray: &mut Self| {
                    let new_tun = !tray.tun_enabled;
                    tray.tun_enabled = new_tun;
                    let engine = Arc::clone(&tray.engine);
                    tray.tokio_handle.spawn(async move {
                        let _ = engine.toggle_tun(new_tun).await;
                    });
                }),
                ..Default::default()
            }
            .into(),
        );

        items.push(MenuItem::Separator);

        // 5. Quit
        items.push(
            StandardItem {
                label: "❌ Çıkış".into(),
                activate: Box::new(|tray: &mut Self| {
                    let engine = Arc::clone(&tray.engine);
                    tray.tokio_handle.block_on(async move {
                        let _ = engine.disconnect().await;
                    });
                    std::process::exit(0);
                }),
                ..Default::default()
            }
            .into(),
        );

        items
    }
}

pub async fn run_tray(db: Database) -> anyhow::Result<()> {
    info!("Starting Throne System Tray (Rust / StatusNotifierItem)...");

    let tokio_handle = TokioHandle::current();
    let db_arc = Arc::new(db);
    let engine = Arc::new(ThroneEngine::new((*db_arc).clone()));

    let settings = db_arc.settings.load_settings().unwrap_or_default();
    let (active_profile_id, active_profile_name) = if let Some(pid) = settings.active_profile_id {
        let name = db_arc
            .profiles
            .get(pid)
            .ok()
            .flatten()
            .map(|p| p.name)
            .unwrap_or_else(|| "Bilinmeyen".into());
        (Some(pid), name)
    } else {
        (None, "Bağlantı Yok".into())
    };

    let tray = ThroneTray {
        db: Arc::clone(&db_arc),
        engine,
        tokio_handle,
        is_connected: settings.active_profile_id.is_some(),
        active_profile_id,
        active_profile_name,
        tun_enabled: settings.enable_tun,
        byedpi_preset: settings.byedpi_preset,
    };

    let _handle = tray.spawn().await?;

    info!("Throne System Tray is active in KDE Plasma taskbar. Waiting for events...");

    // Wait for shutdown signal or keep running
    tokio::signal::ctrl_c().await?;
    info!("Shutdown requested. Stopping Throne Tray...");

    Ok(())
}
