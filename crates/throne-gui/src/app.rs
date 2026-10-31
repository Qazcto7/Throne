use crate::state::{GuiCommand, GuiEvent, StatusState};
use egui::{vec2, Align, Button, Color32, Layout, RichText, ScrollArea, Ui};
use egui_extras::{Column, TableBuilder};
use egui_plot::{Legend, Line, Plot, PlotPoints};
use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use throne_core::{Group, OutboundConfig, Profile, Settings};
use tokio::sync::mpsc::UnboundedSender;

#[derive(PartialEq, Clone, Copy)]
enum BottomTab {
    Logs,
    Connections,
    TrafficGraph,
    RuntimeStats,
}

pub struct ThroneApp {
    cmd_tx: UnboundedSender<GuiCommand>,
    evt_rx: Receiver<GuiEvent>,

    // State
    status: StatusState,
    settings: Settings,
    profiles: Vec<Profile>,
    groups: Vec<Group>,
    active_group_id: i64,
    selected_profile_id: Option<i64>,

    // UI State
    active_bottom_tab: BottomTab,
    log_lines: Vec<String>,
    auto_scroll_logs: bool,
    show_settings_dialog: bool,
    show_import_dialog: bool,
    import_input_text: String,
    search_query: String,

    // Traffic Graph History
    up_speed_history: VecDeque<[f64; 2]>,
    down_speed_history: VecDeque<[f64; 2]>,
    history_time_sec: f64,
    current_up_bytes: f64,
    current_down_bytes: f64,
}

impl ThroneApp {
    pub fn new(
        cmd_tx: UnboundedSender<GuiCommand>,
        evt_rx: Receiver<GuiEvent>,
        _cc: &eframe::CreationContext<'_>,
    ) -> Self {
        Self {
            cmd_tx,
            evt_rx,
            status: StatusState {
                running: false,
                byedpi_running: false,
                active_profile_id: None,
                tun_enabled: false,
                system_proxy: false,
                inbound_port: 2080,
                inbound_address: "127.0.0.1".into(),
            },
            settings: Settings::default(),
            profiles: Vec::new(),
            groups: vec![Group {
                id: 1,
                name: "Default".into(),
                ..Default::default()
            }],
            active_group_id: 1,
            selected_profile_id: None,
            active_bottom_tab: BottomTab::Logs,
            log_lines: Vec::new(),
            auto_scroll_logs: true,
            show_settings_dialog: false,
            show_import_dialog: false,
            import_input_text: String::new(),
            search_query: String::new(),
            up_speed_history: VecDeque::new(),
            down_speed_history: VecDeque::new(),
            history_time_sec: 0.0,
            current_up_bytes: 0.0,
            current_down_bytes: 0.0,
        }
    }

    fn poll_events(&mut self) {
        while let Ok(evt) = self.evt_rx.try_recv() {
            match evt {
                GuiEvent::StatusUpdated(st) => {
                    self.status = st;
                    if self.selected_profile_id.is_none() {
                        self.selected_profile_id = self.status.active_profile_id;
                    }
                }
                GuiEvent::LogAppended(line) => {
                    self.log_lines.push(line);
                    if self.log_lines.len() > 1000 {
                        self.log_lines.remove(0);
                    }
                }
                GuiEvent::ProfilesUpdated(profs) => {
                    self.profiles = profs;
                    if self.selected_profile_id.is_none() && !self.profiles.is_empty() {
                        self.selected_profile_id = Some(self.profiles[0].id);
                    }
                }
                GuiEvent::GroupsUpdated(grps) => {
                    if !grps.is_empty() {
                        self.groups = grps;
                    }
                }
                GuiEvent::SettingsUpdated(st) => {
                    self.settings = *st;
                }
                GuiEvent::LatencyUpdated(pid, lat) => {
                    if let Some(p) = self.profiles.iter_mut().find(|p| p.id == pid) {
                        p.latency = lat.unwrap_or(0);
                    }
                }
                GuiEvent::SpeedSample {
                    up_bytes,
                    down_bytes,
                } => {
                    self.current_up_bytes = up_bytes;
                    self.current_down_bytes = down_bytes;
                    self.history_time_sec += 1.0;
                    let t = self.history_time_sec;

                    self.up_speed_history.push_back([t, up_bytes / 1024.0]);
                    self.down_speed_history.push_back([t, down_bytes / 1024.0]);

                    if self.up_speed_history.len() > 60 {
                        self.up_speed_history.pop_front();
                    }
                    if self.down_speed_history.len() > 60 {
                        self.down_speed_history.pop_front();
                    }
                }
            }
        }
    }

    fn format_bytes(bytes: i64) -> String {
        let b = bytes as f64;
        if b >= 1024.0 * 1024.0 * 1024.0 {
            format!("{:.2} GiB", b / (1024.0 * 1024.0 * 1024.0))
        } else if b >= 1024.0 * 1024.0 {
            format!("{:.2} MiB", b / (1024.0 * 1024.0))
        } else if b >= 1024.0 {
            format!("{:.2} KiB", b / 1024.0)
        } else {
            format!("{} B", bytes)
        }
    }

    fn format_speed(bps: f64) -> String {
        if bps >= 1024.0 * 1024.0 {
            format!("{:.2} MB/s", bps / (1024.0 * 1024.0))
        } else if bps >= 1024.0 {
            format!("{:.2} KB/s", bps / 1024.0)
        } else {
            format!("{:.0} B/s", bps)
        }
    }

    fn render_top_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 0.0);

            // Menu Buttons (Matching screenshot)
            ui.menu_button("Program ▾", |ui| {
                if ui.button("📥 İçe Aktar (Import Link)...").clicked() {
                    self.show_import_dialog = true;
                    ui.close_menu();
                }
                if ui.button("🔄 Verileri Yenile (Refresh)").clicked() {
                    let _ = self.cmd_tx.send(GuiCommand::RefreshData);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("❌ Çıkış (Exit)").clicked() {
                    let _ = self.cmd_tx.send(GuiCommand::Disconnect);
                    std::process::exit(0);
                }
            });

            if ui.button("Settings ⚙").clicked() {
                self.show_settings_dialog = true;
            }

            ui.menu_button("Groups 📋", |ui| {
                for g in &self.groups {
                    let text = if g.id == self.active_group_id {
                        format!("• {}", g.name)
                    } else {
                        g.name.clone()
                    };
                    if ui.button(text).clicked() {
                        self.active_group_id = g.id;
                        ui.close_menu();
                    }
                }
            });

            ui.menu_button("Tools 🛠", |ui| {
                if ui.button("⏱ Hepsini Test Et (Test All)").clicked() {
                    for p in &self.profiles {
                        let _ = self.cmd_tx.send(GuiCommand::TestProfile(p.id));
                    }
                    ui.close_menu();
                }
                if ui.button("🔄 Abonelikleri Yenile").clicked() {
                    let _ = self.cmd_tx.send(GuiCommand::UpdateSubscription(self.active_group_id));
                    ui.close_menu();
                }
            });

            ui.separator();

            // Big Action Button (Play / Stop)
            let is_running = self.status.running;
            let btn_text = if is_running {
                RichText::new("⏹ Durdur").color(Color32::from_rgb(248, 81, 73)).strong()
            } else {
                RichText::new("▶ Başlat").color(Color32::from_rgb(46, 160, 67)).strong()
            };

            let btn = Button::new(btn_text).min_size(vec2(85.0, 26.0));
            if ui.add(btn).clicked() {
                if is_running {
                    let _ = self.cmd_tx.send(GuiCommand::Disconnect);
                } else if let Some(pid) = self.selected_profile_id {
                    let _ = self.cmd_tx.send(GuiCommand::Connect(pid));
                } else if !self.profiles.is_empty() {
                    let _ = self.cmd_tx.send(GuiCommand::Connect(self.profiles[0].id));
                }
            }

            ui.separator();

            // Quick Checkboxes
            let mut tun_val = self.status.tun_enabled;
            if ui.checkbox(&mut tun_val, "Tun Mode").changed() {
                let _ = self.cmd_tx.send(GuiCommand::ToggleTun(tun_val));
            }

            let mut sysproxy_val = self.status.system_proxy;
            if ui.checkbox(&mut sysproxy_val, "System Proxy").changed() {
                let _ = self.cmd_tx.send(GuiCommand::ToggleSystemProxy(sysproxy_val));
            }

            let mut byedpi_active = self.settings.byedpi_enable;
            if ui.checkbox(&mut byedpi_active, "ByeDPI (Vodafone)").changed() {
                let _ = self.cmd_tx.send(GuiCommand::ToggleByeDpi(byedpi_active));
                if byedpi_active && self.settings.byedpi_preset != "vodafone" {
                    let _ = self.cmd_tx.send(GuiCommand::SetByeDpiPreset("vodafone".into()));
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("🔍 Filtrele...")
                        .desired_width(120.0),
                );
            });
        });
    }

    fn render_group_tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
            for g in &self.groups {
                let is_active = g.id == self.active_group_id;
                let text = RichText::new(&g.name).color(if is_active {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(139, 148, 158)
                });

                let fill = if is_active {
                    Color32::from_rgb(43, 114, 214)
                } else {
                    Color32::from_rgb(25, 32, 44)
                };

                let btn = Button::new(text)
                    .fill(fill)
                    .corner_radius(egui::CornerRadius::same(4))
                    .min_size(vec2(60.0, 22.0));

                if ui.add(btn).clicked() {
                    self.active_group_id = g.id;
                }
            }
        });
    }

    fn render_profiles_table(&mut self, ui: &mut Ui) {
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
        let row_height = text_height + 10.0;

        let filtered: Vec<&Profile> = self
            .profiles
            .iter()
            .filter(|p| {
                if self.active_group_id != 1 && p.gid != self.active_group_id {
                    return false;
                }
                if !self.search_query.is_empty() {
                    let q = self.search_query.to_lowercase();
                    return p.name.to_lowercase().contains(&q)
                        || p.profile_type.to_lowercase().contains(&q)
                        || p.outbound_json.to_lowercase().contains(&q);
                }
                true
            })
            .collect();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::exact(35.0))              // #
            .column(Column::initial(70.0).at_least(60.0))   // Type
            .column(Column::initial(160.0).at_least(100.0)) // Address
            .column(Column::remainder().at_least(180.0))    // Name
            .column(Column::initial(90.0).at_least(70.0))   // Test Result
            .column(Column::initial(180.0).at_least(140.0)) // Traffic
            .header(24.0, |mut header| {
                header.col(|ui| { ui.strong("#"); });
                header.col(|ui| { ui.strong("Type"); });
                header.col(|ui| { ui.strong("Address"); });
                header.col(|ui| { ui.strong("Name"); });
                header.col(|ui| { ui.strong("Test Result"); });
                header.col(|ui| { ui.strong("Traffic"); });
            })
            .body(|mut body| {
                for (idx, p) in filtered.iter().enumerate() {
                    let is_active_profile = self.status.running && self.status.active_profile_id == Some(p.id);
                    let is_selected = self.selected_profile_id == Some(p.id);

                    // Parse address
                    let addr = if let Ok(cfg) = serde_json::from_str::<OutboundConfig>(&p.outbound_json) {
                        let (s, port) = cfg.server_and_port();
                        format!("{}:{}", s, port)
                    } else {
                        "-".into()
                    };

                    let traffic_str = format!(
                        "{} ↑  {} ↓",
                        Self::format_bytes(p.traffic_up),
                        Self::format_bytes(p.traffic_dl)
                    );

                    let latency_str = if p.latency > 0 {
                        format!("{} ms", p.latency)
                    } else {
                        String::new()
                    };

                    body.row(row_height, |mut row| {
                        // 1. # Index
                        row.col(|ui| {
                            let label = if is_active_profile {
                                RichText::new(format!("🟢 {}", idx + 1)).color(Color32::from_rgb(63, 185, 80)).strong()
                            } else {
                                RichText::new(format!("{}", idx + 1))
                            };
                            let resp = ui.selectable_label(is_selected, label);
                            if resp.clicked() {
                                self.selected_profile_id = Some(p.id);
                            }
                            if resp.double_clicked() {
                                self.selected_profile_id = Some(p.id);
                                let _ = self.cmd_tx.send(GuiCommand::Connect(p.id));
                            }
                        });

                        // 2. Type
                        row.col(|ui| {
                            let resp = ui.selectable_label(
                                is_selected,
                                RichText::new(&p.profile_type).color(Color32::from_rgb(88, 166, 255)),
                            );
                            if resp.clicked() {
                                self.selected_profile_id = Some(p.id);
                            }
                        });

                        // 3. Address
                        row.col(|ui| {
                            let resp = ui.selectable_label(is_selected, &addr);
                            if resp.clicked() {
                                self.selected_profile_id = Some(p.id);
                            }
                        });

                        // 4. Name
                        row.col(|ui| {
                            let text = if is_active_profile {
                                RichText::new(&p.name).color(Color32::from_rgb(63, 185, 80)).strong()
                            } else {
                                RichText::new(&p.name).color(Color32::WHITE)
                            };
                            let resp = ui.selectable_label(is_selected, text);
                            if resp.clicked() {
                                self.selected_profile_id = Some(p.id);
                            }
                            if resp.double_clicked() {
                                self.selected_profile_id = Some(p.id);
                                let _ = self.cmd_tx.send(GuiCommand::Connect(p.id));
                            }
                            resp.context_menu(|ui| {
                                if ui.button("⚡ Bağlan").clicked() {
                                    let _ = self.cmd_tx.send(GuiCommand::Connect(p.id));
                                    ui.close_menu();
                                }
                                if ui.button("⏱ Test Et").clicked() {
                                    let _ = self.cmd_tx.send(GuiCommand::TestProfile(p.id));
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("🗑 Profili Sil").clicked() {
                                    let _ = self.cmd_tx.send(GuiCommand::DeleteProfile(p.id));
                                    ui.close_menu();
                                }
                            });
                        });

                        // 5. Test Result
                        row.col(|ui| {
                            let resp = ui.selectable_label(
                                is_selected,
                                RichText::new(&latency_str).color(Color32::from_rgb(227, 179, 65)),
                            );
                            if resp.clicked() {
                                self.selected_profile_id = Some(p.id);
                            }
                        });

                        // 6. Traffic
                        row.col(|ui| {
                            let resp = ui.selectable_label(
                                is_selected,
                                RichText::new(&traffic_str).color(Color32::from_rgb(139, 148, 158)),
                            );
                            if resp.clicked() {
                                self.selected_profile_id = Some(p.id);
                            }
                        });
                    });
                }
            });
    }

    fn render_bottom_tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 0.0);

            let tabs = [
                (BottomTab::Logs, "Logs"),
                (BottomTab::Connections, "Connections"),
                (BottomTab::TrafficGraph, "Traffic Graph"),
                (BottomTab::RuntimeStats, "Runtime Stats"),
            ];

            for (tab, title) in tabs {
                let is_active = self.active_bottom_tab == tab;
                let text = RichText::new(title).color(if is_active {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(139, 148, 158)
                });

                let fill = if is_active {
                    Color32::from_rgb(33, 42, 58)
                } else {
                    Color32::from_rgb(18, 22, 31)
                };

                let btn = Button::new(text)
                    .fill(fill)
                    .corner_radius(egui::CornerRadius::same(3))
                    .min_size(vec2(75.0, 22.0));

                if ui.add(btn).clicked() {
                    self.active_bottom_tab = tab;
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.active_bottom_tab == BottomTab::Logs && ui.button("Temizle").clicked() {
                    self.log_lines.clear();
                }
            });
        });

        ui.separator();

        match self.active_bottom_tab {
            BottomTab::Logs => {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(self.auto_scroll_logs)
                    .show(ui, |ui| {
                        ui.style_mut().override_font_id = Some(egui::FontId::monospace(12.0));
                        for line in &self.log_lines {
                            let color = if line.contains("[ERROR]") {
                                Color32::from_rgb(248, 81, 73)
                            } else if line.contains("[SUCCESS]") || line.contains("Core Has Successfully") {
                                Color32::from_rgb(63, 185, 80)
                            } else if line.contains("[INIT]") || line.contains("[CONNECT]") {
                                Color32::from_rgb(88, 166, 255)
                            } else {
                                Color32::from_rgb(201, 209, 217)
                            };
                            ui.colored_label(color, line);
                        }
                    });
            }
            BottomTab::TrafficGraph => {
                let up_pts: PlotPoints = self.up_speed_history.iter().copied().collect();
                let down_pts: PlotPoints = self.down_speed_history.iter().copied().collect();

                let up_line = Line::new(up_pts)
                    .color(Color32::from_rgb(43, 114, 214))
                    .name("Yükleme (KB/s)");
                let down_line = Line::new(down_pts)
                    .color(Color32::from_rgb(46, 160, 67))
                    .name("İndirme (KB/s)");

                Plot::new("traffic_plot")
                    .show_axes([false, true])
                    .legend(Legend::default())
                    .height(ui.available_height() - 5.0)
                    .show(ui, |plot_ui| {
                        plot_ui.line(down_line);
                        plot_ui.line(up_line);
                    });
            }
            BottomTab::Connections => {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    if self.status.running {
                        ui.heading("Aktif Tünel Bağlantıları");
                        ui.label(format!("Inbound Dinleyici: 127.0.0.1:{}", self.status.inbound_port));
                        ui.label(format!("DPI Atlama: 127.0.0.1:1080 (ciadpi - {})", self.settings.byedpi_preset));
                    } else {
                        ui.label("Bağlantı aktif değil.");
                    }
                });
            }
            BottomTab::RuntimeStats => {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading("Çalışma İstatistikleri");
                        ui.label("Throne Sürümü: 1.4.0 (Rust 2024 Edition)");
                        ui.label(format!("ByeDPI Durumu: {}", if self.status.byedpi_running { "Çalışıyor" } else { "Durduruldu" }));
                        ui.label(format!("TUN Durumu: {}", if self.status.tun_enabled { "Açık" } else { "Kapalı" }));
                        ui.label(format!("Sistem Vekili: {}", if self.status.system_proxy { "Açık" } else { "Kapalı" }));
                    });
                });
            }
        }
    }

    fn render_status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(16.0, 0.0);

            // Left: Status
            if self.status.running {
                let name = self
                    .status
                    .active_profile_id
                    .and_then(|id| self.profiles.iter().find(|p| p.id == id).map(|p| p.name.as_str()))
                    .unwrap_or("Aktif Profil");
                ui.colored_label(
                    Color32::from_rgb(63, 185, 80),
                    format!("🟢 Running: {}", name),
                );
            } else {
                ui.colored_label(Color32::from_rgb(139, 148, 158), "⚪ Not Running");
            }

            ui.separator();

            // Center: Mixed & ByeDPI port
            ui.colored_label(
                Color32::from_rgb(139, 148, 158),
                format!("Mixed: {}:{}", self.status.inbound_address, self.status.inbound_port),
            );

            if self.status.byedpi_running {
                ui.colored_label(
                    Color32::from_rgb(88, 166, 255),
                    "ByeDPI: 127.0.0.1:1080 (Vodafone)",
                );
            }

            // Right: Speeds
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(format!(
                    "Proxy: {} ↑  {} ↓",
                    Self::format_speed(self.current_up_bytes),
                    Self::format_speed(self.current_down_bytes)
                ));
            });
        });
    }

    fn render_import_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_import_dialog {
            return;
        }

        egui::Window::new("İçe Aktar (Import Nodes)")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("VLESS, VMess, Shadowsocks bağlantısı veya Abonelik URL'si girin:");
                ui.add(
                    egui::TextEdit::multiline(&mut self.import_input_text)
                        .desired_rows(6)
                        .desired_width(450.0),
                );

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("İçe Aktar").clicked() {
                        let text = self.import_input_text.trim().to_string();
                        if !text.is_empty() {
                            let _ = self.cmd_tx.send(GuiCommand::ImportSource(text, self.active_group_id));
                            self.import_input_text.clear();
                            self.show_import_dialog = false;
                        }
                    }
                    if ui.button("İptal").clicked() {
                        self.show_import_dialog = false;
                    }
                });
            });
    }

    fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_settings_dialog {
            return;
        }

        let mut open = true;
        egui::Window::new("Ayarlar (Settings)")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_width(380.0);
                egui::Grid::new("settings_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                    ui.label("Inbound Port:");
                    ui.add(egui::DragValue::new(&mut self.settings.inbound_port));
                    ui.end_row();

                    ui.label("TUN Arayüzü:");
                    ui.text_edit_singleline(&mut self.settings.tun_interface_name);
                    ui.end_row();

                    ui.label("Uzak DNS:");
                    ui.text_edit_singleline(&mut self.settings.remote_dns);
                    ui.end_row();

                    ui.label("Doğrudan DNS:");
                    ui.text_edit_singleline(&mut self.settings.direct_dns);
                    ui.end_row();

                    ui.label("ByeDPI Komutu / Argüman:");
                    ui.text_edit_singleline(&mut self.settings.byedpi_args);
                    ui.end_row();
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Kaydet").clicked() {
                        let _ = self.cmd_tx.send(GuiCommand::SaveSettings(Box::new(self.settings.clone())));
                        self.show_settings_dialog = false;
                    }
                    if ui.button("Kapat").clicked() {
                        self.show_settings_dialog = false;
                    }
                });
            });

        if !open {
            self.show_settings_dialog = false;
        }
    }
}

impl eframe::App for ThroneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();

        // Top Panel: Toolbar
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
                self.render_top_bar(ui);
                ui.add_space(6.0);
                self.render_group_tabs(ui);
            });

        // Bottom Panel: Status bar
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(6.0))
            .show(ctx, |ui| {
                self.render_status_bar(ui);
            });

        // Bottom Split Panel: Logs, Connections, Graph, Stats
        egui::TopBottomPanel::bottom("bottom_tabs_panel")
            .resizable(true)
            .default_height(220.0)
            .min_height(120.0)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
                self.render_bottom_tabs(ui);
            });

        // Central Panel: Profiles Table
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(6.0))
            .show(ctx, |ui| {
                self.render_profiles_table(ui);
            });

        // Dialogs
        self.render_import_dialog(ctx);
        self.render_settings_dialog(ctx);

        // Keep UI reactive: request repaint every 500ms for status/graphs
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}
