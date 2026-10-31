use crate::byedpi_runner::{ByeDpiPreset, ByeDpiRunner};
use crate::core_runner::CoreRunner;
use crate::latency::measure_tcp_latency;
use crate::sub_updater::SubUpdater;
use crate::system_proxy::SystemProxyManager;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use throne_api::{ApiHandler, ApiRequest, ApiResponse};
use throne_core::{OutboundConfig, SingBoxGenerator};
use throne_db::Database;
use tracing::info;

pub struct ThroneEngine {
    pub db: Arc<Database>,
    pub runner: Arc<CoreRunner>,
    pub byedpi: Arc<ByeDpiRunner>,
    pub sub_updater: Arc<SubUpdater>,
}

impl ThroneEngine {
    pub fn new(db: Database) -> Self {
        let db_arc = Arc::new(db);
        let settings = db_arc.settings.load_settings().unwrap_or_default();
        let runner = Arc::new(CoreRunner::new(settings.core_path));
        let byedpi = Arc::new(ByeDpiRunner::new(settings.byedpi_path));
        let sub_updater = Arc::new(SubUpdater::new(Arc::clone(&db_arc)));

        Self {
            db: db_arc,
            runner,
            byedpi,
            sub_updater,
        }
    }

    pub async fn connect(&self, profile_id: i64) -> anyhow::Result<()> {
        self.connect_with_options(profile_id, None, None).await
    }

    pub async fn connect_with_options(
        &self,
        profile_id: i64,
        force_byedpi: Option<bool>,
        byedpi_args: Option<Vec<String>>,
    ) -> anyhow::Result<()> {
        let profile = self
            .db
            .profiles
            .get(profile_id)?
            .ok_or_else(|| anyhow::anyhow!("Profile {} not found", profile_id))?;

        let mut settings = self.db.settings.load_settings()?;

        // Determine if ByeDPI is needed
        let needs_byedpi = if let Some(forced) = force_byedpi {
            forced
        } else if settings.byedpi_enable {
            true
        } else {
            // Auto-detection:
            // 1) Outbound targets 127.0.0.1:1080 (ByeDPI SOCKS listener)
            // 2) Profile name mentions discord / roblox / byedpi
            let is_local_1080 = profile.outbound_json.contains("127.0.0.1")
                && (profile.outbound_json.contains("1080") || profile.profile_type == "socks");
            let name_hint = profile.name.to_lowercase();
            let matches_hint = name_hint.contains("byedpi")
                || name_hint.contains("discord")
                || name_hint.contains("roblox");
            is_local_1080 || matches_hint
        };

        if needs_byedpi {
            info!("Profile requires ByeDPI (DPI circumvention). Starting ByeDPI supervisor...");
            let args = byedpi_args.unwrap_or_else(|| {
                if settings.byedpi_preset == "vodafone" || settings.byedpi_preset == "vf" {
                    ByeDpiPreset::Vodafone.to_args()
                } else if !settings.byedpi_args.is_empty() {
                    settings
                        .byedpi_args
                        .split_whitespace()
                        .map(String::from)
                        .collect()
                } else {
                    ByeDpiPreset::Standard.to_args()
                }
            });

            if let Err(e) = self.byedpi.start(None, &args).await {
                tracing::warn!("ByeDPI start warning: {}. Proceeding with core...", e);
            }
        }

        let route_profile = match settings.active_route_id {
            Some(id) => self.db.routes.get_profile(id)?,
            None => None,
        };

        let route_rules = if let Some(ref rp) = route_profile {
            self.db.routes.list_rules_for_profile(rp.id)?
        } else {
            Vec::new()
        };

        let generator = SingBoxGenerator::new(
            &settings,
            &profile,
            route_profile.as_ref(),
            &route_rules,
        );
        let config_json = generator.generate()?;

        self.runner.start(&config_json).await?;

        settings.active_profile_id = Some(profile_id);
        self.db.settings.save_settings(&settings)?;

        if settings.system_proxy || settings.enable_tun {
            let _ = SystemProxyManager::enable(&settings.inbound_address, settings.inbound_port);
        }

        info!(
            "Successfully connected to profile: {} ({})",
            profile.name, profile.id
        );
        Ok(())
    }

    pub async fn disconnect(&self) -> anyhow::Result<()> {
        self.runner.stop().await?;
        self.byedpi.stop().await?;

        let mut settings = self.db.settings.load_settings()?;
        settings.active_profile_id = None;
        self.db.settings.save_settings(&settings)?;

        let _ = SystemProxyManager::disable();

        info!("Disconnected successfully");
        Ok(())
    }

    pub async fn status(&self) -> anyhow::Result<serde_json::Value> {
        let running = self.runner.is_running().await;
        let byedpi_running = self.byedpi.is_running().await;
        let settings = self.db.settings.load_settings()?;

        let active_profile = match settings.active_profile_id {
            Some(id) => self.db.profiles.get(id)?,
            None => None,
        };

        Ok(json!({
            "running": running,
            "byedpi_running": byedpi_running,
            "active_profile": active_profile,
            "tun_enabled": settings.enable_tun,
            "system_proxy": settings.system_proxy,
            "inbound_port": settings.inbound_port,
            "inbound_address": settings.inbound_address,
            "version": "1.4.0 (Rust)"
        }))
    }

    pub async fn test_profile(&self, profile_id: i64) -> anyhow::Result<Option<i32>> {
        let profile = self
            .db
            .profiles
            .get(profile_id)?
            .ok_or_else(|| anyhow::anyhow!("Profile {} not found", profile_id))?;

        let cfg: OutboundConfig = serde_json::from_str(&profile.outbound_json)?;
        let (server, port) = cfg.server_and_port();

        let latency = measure_tcp_latency(server, port, 3000).await;
        if let Some(rtt) = latency {
            self.db.profiles.update_latency(profile_id, rtt)?;
        }

        Ok(latency)
    }

    pub async fn toggle_tun(&self, enable: bool) -> anyhow::Result<()> {
        let mut settings = self.db.settings.load_settings()?;
        settings.enable_tun = enable;
        self.db.settings.save_settings(&settings)?;

        // Only reconnect if currently running!
        if self.runner.is_running().await {
            if enable {
                let _ = SystemProxyManager::enable(&settings.inbound_address, settings.inbound_port);
            }
            if let Some(active_id) = settings.active_profile_id {
                self.connect(active_id).await?;
            }
        }
        Ok(())
    }

    pub async fn toggle_system_proxy(&self, enable: bool) -> anyhow::Result<()> {
        let mut settings = self.db.settings.load_settings()?;
        settings.system_proxy = enable;
        self.db.settings.save_settings(&settings)?;

        if enable && self.runner.is_running().await {
            SystemProxyManager::enable(&settings.inbound_address, settings.inbound_port)?;
        } else {
            SystemProxyManager::disable()?;
        }
        Ok(())
    }
}

#[async_trait]
impl ApiHandler for ThroneEngine {
    async fn handle(&self, req: ApiRequest) -> ApiResponse {
        let id = req.id;
        match req.method.as_str() {
            "status" => match self.status().await {
                Ok(val) => ApiResponse::success(id, val),
                Err(e) => ApiResponse::err(id, e.to_string()),
            },
            "connect" => {
                let pid = req
                    .params
                    .and_then(|p| p.get("profile_id").and_then(|v| v.as_i64()));
                match pid {
                    Some(profile_id) => match self.connect(profile_id).await {
                        Ok(_) => ApiResponse::success(id, json!({ "status": "connected" })),
                        Err(e) => ApiResponse::err(id, e.to_string()),
                    },
                    None => ApiResponse::err(id, "Missing profile_id param"),
                }
            }
            "disconnect" => match self.disconnect().await {
                Ok(_) => ApiResponse::success(id, json!({ "status": "disconnected" })),
                Err(e) => ApiResponse::err(id, e.to_string()),
            },
            "profiles" => match self.db.profiles.list_all() {
                Ok(profiles) => match self.db.groups.list_all() {
                    Ok(groups) => ApiResponse::success(
                        id,
                        json!({
                            "profiles": profiles,
                            "groups": groups
                        }),
                    ),
                    Err(e) => ApiResponse::err(id, e.to_string()),
                },
                Err(e) => ApiResponse::err(id, e.to_string()),
            },
            "tun" => {
                let enable = req
                    .params
                    .and_then(|p| p.get("enable").and_then(|v| v.as_bool()))
                    .unwrap_or(false);
                match self.toggle_tun(enable).await {
                    Ok(_) => ApiResponse::success(id, json!({ "tun": enable })),
                    Err(e) => ApiResponse::err(id, e.to_string()),
                }
            }
            "system_proxy" => {
                let enable = req
                    .params
                    .and_then(|p| p.get("enable").and_then(|v| v.as_bool()))
                    .unwrap_or(false);
                match self.toggle_system_proxy(enable).await {
                    Ok(_) => ApiResponse::success(id, json!({ "system_proxy": enable })),
                    Err(e) => ApiResponse::err(id, e.to_string()),
                }
            }
            "test" => {
                let pid = req
                    .params
                    .and_then(|p| p.get("profile_id").and_then(|v| v.as_i64()));
                if let Some(profile_id) = pid {
                    match self.test_profile(profile_id).await {
                        Ok(latency) => ApiResponse::success(id, json!({ "latency": latency })),
                        Err(e) => ApiResponse::err(id, e.to_string()),
                    }
                } else {
                    ApiResponse::err(id, "Missing profile_id param")
                }
            }
            "update_subscription" => {
                let gid = req
                    .params
                    .and_then(|p| p.get("group_id").and_then(|v| v.as_i64()))
                    .unwrap_or(1);
                match self.sub_updater.update_group(gid).await {
                    Ok(count) => ApiResponse::success(id, json!({ "imported_nodes": count })),
                    Err(e) => ApiResponse::err(id, e.to_string()),
                }
            }
            "byedpi_start" => {
                let preset = req
                    .params
                    .as_ref()
                    .and_then(|p| p.get("preset").and_then(|v| v.as_str()))
                    .unwrap_or("standard");
                let preset_obj = ByeDpiPreset::from_str_name(preset);
                match self.byedpi.start(None, &preset_obj.to_args()).await {
                    Ok(_) => ApiResponse::success(
                        id,
                        json!({ "status": "started", "preset": preset }),
                    ),
                    Err(e) => ApiResponse::err(id, e.to_string()),
                }
            }
            "byedpi_stop" => match self.byedpi.stop().await {
                Ok(_) => ApiResponse::success(id, json!({ "status": "stopped" })),
                Err(e) => ApiResponse::err(id, e.to_string()),
            },
            "byedpi_status" => {
                let running = self.byedpi.is_running().await;
                ApiResponse::success(id, json!({ "running": running }))
            }
            other => ApiResponse::err(id, format!("Unknown method: {other}")),
        }
    }
}
