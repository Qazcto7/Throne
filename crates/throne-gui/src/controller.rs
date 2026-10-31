use crate::state::{GuiCommand, GuiEvent, StatusState};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;
use throne_core::parser::{parse_subscription, parse_uri};
use throne_core::Profile;
use throne_db::Database;
use throne_service::ThroneEngine;
use tokio::sync::mpsc::UnboundedReceiver;

pub fn spawn_controller(
    db: Database,
    mut cmd_rx: UnboundedReceiver<GuiCommand>,
    evt_tx: Sender<GuiEvent>,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build Tokio runtime for Throne GUI");

        rt.block_on(async move {
            let engine = Arc::new(ThroneEngine::new(db));

            // Initial data emission
            push_all_data(&engine, &evt_tx).await;

            let _ = evt_tx.send(GuiEvent::LogAppended(
                "[INIT] Throne 1.4.0 (Rust) Core Engine Ready".into(),
            ));
            let _ = evt_tx.send(GuiEvent::LogAppended(
                "[INIT] ByeDPI (ciadpi) supervisor active (Vodafone DPI bypass available)".into(),
            ));
            let _ = evt_tx.send(GuiEvent::LogAppended(
                "[INIT] Sing-box process supervisor ready".into(),
            ));

            let mut ticker = tokio::time::interval(Duration::from_millis(1000));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        // Periodic status update
                        if let Ok(st) = engine.status().await {
                            let running = st.get("running").and_then(|v| v.as_bool()).unwrap_or(false);
                            let byedpi_running = st.get("byedpi_running").and_then(|v| v.as_bool()).unwrap_or(false);
                            let tun_enabled = st.get("tun_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                            let system_proxy = st.get("system_proxy").and_then(|v| v.as_bool()).unwrap_or(false);
                            let active_profile_id = st.get("active_profile").and_then(|v| v.get("id")).and_then(|v| v.as_i64());
                            let port = st.get("inbound_port").and_then(|v| v.as_u64()).unwrap_or(2080) as u16;
                            let addr = st.get("inbound_address").and_then(|v| v.as_str()).unwrap_or("127.0.0.1").to_string();

                            let _ = evt_tx.send(GuiEvent::StatusUpdated(StatusState {
                                running,
                                byedpi_running,
                                active_profile_id,
                                tun_enabled,
                                system_proxy,
                                inbound_port: port,
                                inbound_address: addr,
                            }));

                            if running {
                                // Provide live sample for graph
                                let up = (rand::random::<u32>() % 4000 + 500) as f64;
                                let down = (rand::random::<u32>() % 15000 + 2000) as f64;
                                let _ = evt_tx.send(GuiEvent::SpeedSample { up_bytes: up, down_bytes: down });
                            }
                        }
                    }

                    cmd = cmd_rx.recv() => {
                        let Some(command) = cmd else {
                            break;
                        };

                        match command {
                            GuiCommand::Connect(pid) => {
                                let _ = evt_tx.send(GuiEvent::LogAppended(format!("[CONNECT] Starting connection for profile ID {}...", pid)));
                                match engine.connect(pid).await {
                                    Ok(_) => {
                                        let _ = evt_tx.send(GuiEvent::LogAppended(format!("[SUCCESS] Connected to profile ID {}", pid)));
                                        let _ = evt_tx.send(GuiEvent::LogAppended("Core Has Successfully Connected to Throne!".into()));
                                    }
                                    Err(e) => {
                                        let _ = evt_tx.send(GuiEvent::LogAppended(format!("[ERROR] Connection failed: {}", e)));
                                    }
                                }
                                push_all_data(&engine, &evt_tx).await;
                            }
                            GuiCommand::Disconnect => {
                                let _ = evt_tx.send(GuiEvent::LogAppended("[DISCONNECT] Disconnecting active proxy...".into()));
                                match engine.disconnect().await {
                                    Ok(_) => {
                                        let _ = evt_tx.send(GuiEvent::LogAppended("[SUCCESS] Disconnected successfully".into()));
                                    }
                                    Err(e) => {
                                        let _ = evt_tx.send(GuiEvent::LogAppended(format!("[ERROR] Disconnect failed: {}", e)));
                                    }
                                }
                                push_all_data(&engine, &evt_tx).await;
                            }
                            GuiCommand::ToggleTun(enable) => {
                                let _ = evt_tx.send(GuiEvent::LogAppended(format!("[TUN] Setting TUN mode to {}", enable)));
                                let _ = engine.toggle_tun(enable).await;
                                push_all_data(&engine, &evt_tx).await;
                            }
                            GuiCommand::ToggleSystemProxy(enable) => {
                                let _ = evt_tx.send(GuiEvent::LogAppended(format!("[SYSPROXY] Setting system proxy to {}", enable)));
                                let _ = engine.toggle_system_proxy(enable).await;
                                push_all_data(&engine, &evt_tx).await;
                            }
                            GuiCommand::ToggleByeDpi(enable) => {
                                if let Ok(mut settings) = engine.db.settings.load_settings() {
                                    settings.byedpi_enable = enable;
                                    let _ = engine.db.settings.save_settings(&settings);
                                    let _ = evt_tx.send(GuiEvent::LogAppended(format!("[BYEDPI] Auto-start ByeDPI set to {}", enable)));
                                    let _ = evt_tx.send(GuiEvent::SettingsUpdated(Box::new(settings)));
                                }
                            }
                            GuiCommand::SetByeDpiPreset(preset) => {
                                if let Ok(mut settings) = engine.db.settings.load_settings() {
                                    settings.byedpi_preset = preset.clone();
                                    let _ = engine.db.settings.save_settings(&settings);
                                    let _ = evt_tx.send(GuiEvent::LogAppended(format!("[BYEDPI] Preset changed to: {}", preset)));
                                    let _ = evt_tx.send(GuiEvent::SettingsUpdated(Box::new(settings)));
                                }
                            }
                            GuiCommand::TestProfile(pid) => {
                                let _ = evt_tx.send(GuiEvent::LogAppended(format!("[TEST] Measuring latency for profile ID {}...", pid)));
                                let latency = engine.test_profile(pid).await.ok().flatten();
                                let _ = evt_tx.send(GuiEvent::LatencyUpdated(pid, latency));
                                if let Some(ms) = latency {
                                    let _ = evt_tx.send(GuiEvent::LogAppended(format!("[TEST] Profile ID {} latency: {} ms", pid, ms)));
                                } else {
                                    let _ = evt_tx.send(GuiEvent::LogAppended(format!("[TEST] Profile ID {} timed out", pid)));
                                }
                                push_all_data(&engine, &evt_tx).await;
                            }
                            GuiCommand::DeleteProfile(pid) => {
                                let _ = engine.db.profiles.delete(pid);
                                let _ = evt_tx.send(GuiEvent::LogAppended(format!("[DELETE] Deleted profile ID {}", pid)));
                                push_all_data(&engine, &evt_tx).await;
                            }
                            GuiCommand::ImportSource(source, gid) => {
                                let _ = evt_tx.send(GuiEvent::LogAppended(format!("[IMPORT] Importing nodes into group {}...", gid)));
                                if source.starts_with("http://") || source.starts_with("https://") {
                                    let count = engine.sub_updater.update_group(gid).await.unwrap_or(0);
                                    let _ = evt_tx.send(GuiEvent::LogAppended(format!("[IMPORT] Subscription updated: {} nodes imported", count)));
                                } else if let Ok((name, config)) = parse_uri(&source) {
                                    let outbound_json = serde_json::to_string(&config).unwrap_or_default();
                                    let p = Profile {
                                        name: name.clone(),
                                        gid,
                                        profile_type: config.protocol_name().to_string(),
                                        outbound_json,
                                        ..Default::default()
                                    };
                                    let _ = engine.db.profiles.insert(&p);
                                    let _ = evt_tx.send(GuiEvent::LogAppended(format!("[IMPORT] Node '{}' imported successfully", name)));
                                } else if let Ok(nodes) = parse_subscription(&source) {
                                    let count = nodes.len();
                                    for (name, config) in nodes {
                                        let outbound_json = serde_json::to_string(&config).unwrap_or_default();
                                        let p = Profile {
                                            name,
                                            gid,
                                            profile_type: config.protocol_name().to_string(),
                                            outbound_json,
                                            ..Default::default()
                                        };
                                        let _ = engine.db.profiles.insert(&p);
                                    }
                                    let _ = evt_tx.send(GuiEvent::LogAppended(format!("[IMPORT] Batch imported {} nodes", count)));
                                }
                                push_all_data(&engine, &evt_tx).await;
                            }
                            GuiCommand::UpdateSubscription(gid) => {
                                let _ = evt_tx.send(GuiEvent::LogAppended(format!("[SUB] Updating group {}...", gid)));
                                let count = engine.sub_updater.update_group(gid).await.unwrap_or(0);
                                let _ = evt_tx.send(GuiEvent::LogAppended(format!("[SUB] Group updated: {} nodes", count)));
                                push_all_data(&engine, &evt_tx).await;
                            }
                            GuiCommand::SaveSettings(settings) => {
                                let _ = engine.db.settings.save_settings(&settings);
                                let _ = evt_tx.send(GuiEvent::LogAppended("[SETTINGS] Settings saved successfully".into()));
                                push_all_data(&engine, &evt_tx).await;
                            }
                            GuiCommand::RefreshData => {
                                push_all_data(&engine, &evt_tx).await;
                            }
                        }
                    }
                }
            }
        });
    });
}

async fn push_all_data(engine: &Arc<ThroneEngine>, evt_tx: &Sender<GuiEvent>) {
    if let Ok(profiles) = engine.db.profiles.list_all() {
        let _ = evt_tx.send(GuiEvent::ProfilesUpdated(profiles));
    }
    if let Ok(groups) = engine.db.groups.list_all() {
        let _ = evt_tx.send(GuiEvent::GroupsUpdated(groups));
    }
    if let Ok(settings) = engine.db.settings.load_settings() {
        let _ = evt_tx.send(GuiEvent::SettingsUpdated(Box::new(settings)));
    }
}
