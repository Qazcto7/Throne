use crate::state::{GuiCommand, GuiEvent, StatusState};
use egui::{pos2, vec2, Align, Button, Color32, Layout, Rect, RichText, ScrollArea, Stroke, Ui};
use egui_extras::{Column, TableBuilder};
use egui_plot::{Legend, Line, Plot, PlotPoints};
use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use throne_core::{Group, OutboundConfig, Profile, RouteProfile, RouteRule, Settings};
use tokio::sync::mpsc::UnboundedSender;

#[derive(PartialEq, Clone, Copy)]
enum BottomTab {
    Logs,
    Connections,
    TrafficGraph,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum SettingsTab {
    Common,
    Core,
    Dns,
    Tun,
    Routing,
    ByeDpi,
    SystemProxy,
}

pub struct ThroneApp {
    cmd_tx: UnboundedSender<GuiCommand>,
    evt_rx: Receiver<GuiEvent>,

    // State
    status: StatusState,
    settings: Settings,
    profiles: Vec<Profile>,
    groups: Vec<Group>,
    routes: Vec<RouteProfile>,
    active_group_id: i64,
    selected_profile_id: Option<i64>,

    // UI State
    active_bottom_tab: BottomTab,
    active_settings_tab: SettingsTab,
    log_lines: Vec<String>,
    auto_scroll_logs: bool,
    show_settings_dialog: bool,
    show_import_dialog: bool,
    import_input_text: String,
    search_query: String,

    // Routing Dialog State
    show_routing_dialog: bool,
    routing_selected_profile_id: Option<i64>,
    editing_route_profile: Option<RouteProfile>,
    editing_route_rules: Vec<RouteRule>,
    show_rule_editor: bool,
    editing_rule_index: Option<usize>,
    rule_draft: RouteRule,
    rule_domains_text: String,
    rule_suffixes_text: String,
    rule_processes_text: String,
    rule_cidrs_text: String,
    show_new_profile_modal: bool,
    new_profile_name: String,

    // Predefined rules multiline buffer for DNS settings editor
    dns_predefined_text: String,

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
        let default_settings = Settings::default();
        let dns_predefined_text = default_settings.dns_predefined_rules.join("\n");

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
            settings: default_settings,
            profiles: Vec::new(),
            groups: vec![Group {
                id: 1,
                name: "Default".into(),
                ..Default::default()
            }],
            routes: Vec::new(),
            active_group_id: 1,
            selected_profile_id: None,
            active_bottom_tab: BottomTab::Logs,
            active_settings_tab: SettingsTab::Common,
            log_lines: Vec::new(),
            auto_scroll_logs: true,
            show_settings_dialog: false,
            show_import_dialog: false,
            import_input_text: String::new(),
            search_query: String::new(),
            show_routing_dialog: false,
            routing_selected_profile_id: None,
            editing_route_profile: None,
            editing_route_rules: Vec::new(),
            show_rule_editor: false,
            editing_rule_index: None,
            rule_draft: RouteRule::default(),
            rule_domains_text: String::new(),
            rule_suffixes_text: String::new(),
            rule_processes_text: String::new(),
            rule_cidrs_text: String::new(),
            show_new_profile_modal: false,
            new_profile_name: String::new(),
            dns_predefined_text,
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
                GuiEvent::RoutesUpdated(rts) => {
                    self.routes = rts;
                    if self.routing_selected_profile_id.is_none() && !self.routes.is_empty() {
                        let active_rid = self.settings.active_route_id.unwrap_or(self.routes[0].id);
                        let target_profile = self
                            .routes
                            .iter()
                            .find(|r| r.id == active_rid)
                            .cloned()
                            .unwrap_or_else(|| self.routes[0].clone());
                        let target_id = target_profile.id;
                        self.routing_selected_profile_id = Some(target_id);
                        self.editing_route_profile = Some(target_profile);
                        let _ = self.cmd_tx.send(GuiCommand::LoadRouteRules(target_id));
                    }
                }
                GuiEvent::RouteRulesLoaded(rid, rules) => {
                    if self.routing_selected_profile_id == Some(rid) {
                        self.editing_route_rules = rules;
                    }
                }
                GuiEvent::SettingsUpdated(st) => {
                    self.settings = *st;
                    self.dns_predefined_text = self.settings.dns_predefined_rules.join("\n");
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

    fn render_start_stop_button(ui: &mut Ui, is_running: bool) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(vec2(38.0, 38.0), egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let center = rect.center();
            let radius = 17.0;

            // Outer ring color & glow
            let ring_color = if is_running {
                Color32::from_rgb(248, 81, 73) // Glowing Red
            } else {
                Color32::from_rgb(46, 160, 67) // Emerald Green
            };

            let bg_color = if response.hovered() {
                if is_running {
                    Color32::from_rgb(60, 20, 20)
                } else {
                    Color32::from_rgb(20, 48, 28)
                }
            } else {
                if is_running {
                    Color32::from_rgb(38, 16, 16)
                } else {
                    Color32::from_rgb(14, 32, 20)
                }
            };

            // Outer ring
            painter.circle(center, radius, bg_color, Stroke::new(2.5_f32, ring_color));

            // Inner icon
            if is_running {
                // Square stop icon
                let half_sz = 5.5;
                let square_rect = Rect::from_min_max(
                    pos2(center.x - half_sz, center.y - half_sz),
                    pos2(center.x + half_sz, center.y + half_sz),
                );
                painter.rect_filled(square_rect, 1.5, ring_color);
            } else {
                // Triangle play icon pointing right
                let p1 = pos2(center.x - 4.5, center.y - 6.5);
                let p2 = pos2(center.x - 4.5, center.y + 6.5);
                let p3 = pos2(center.x + 6.0, center.y);
                painter.add(egui::Shape::convex_polygon(
                    vec![p1, p2, p3],
                    ring_color,
                    Stroke::NONE,
                ));
            }
        }

        response
    }

    fn render_top_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 0.0);

            // 1. Program Menu
            ui.menu_button("Program ▾", |ui| {
                if ui.button("📥 İçe Aktar (Import Links)...").clicked() {
                    self.show_import_dialog = true;
                    ui.close_menu();
                }
                if ui.button("📋 Panodan İçe Aktar").clicked() {
                    // Try reading clipboard
                    self.show_import_dialog = true;
                    ui.close_menu();
                }
                ui.separator();
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

            // 2. Settings Menu
            ui.menu_button("Settings ▾", |ui| {
                if ui.button("⚙ Tercihler (Preferences)...").clicked() {
                    self.show_settings_dialog = true;
                    ui.close_menu();
                }
                if ui.button("🛡 TUN / VPN Ayarları...").clicked() {
                    self.active_settings_tab = SettingsTab::Tun;
                    self.show_settings_dialog = true;
                    ui.close_menu();
                }
                if ui.button("🌐 DNS Ayarları...").clicked() {
                    self.active_settings_tab = SettingsTab::Dns;
                    self.show_settings_dialog = true;
                    ui.close_menu();
                }
                if ui.button("⚡ ByeDPI / DPI Atlama Ayarları...").clicked() {
                    self.active_settings_tab = SettingsTab::ByeDpi;
                    self.show_settings_dialog = true;
                    ui.close_menu();
                }
                if ui.button("🔀 Yönlendirme Ayarları (Routing)...").clicked() {
                    self.show_routing_dialog = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("📁 Yapılandırma Klasörünü Aç").clicked() {
                    let _ = std::process::Command::new("xdg-open")
                        .arg("/home/kazim/byedpi_throne_linux/Throne/config")
                        .status();
                    ui.close_menu();
                }
            });

            // 3. Groups Menu
            ui.menu_button("Groups ▾", |ui| {
                for g in &self.groups {
                    let text = if g.id == self.active_group_id {
                        format!("✓ {}", g.name)
                    } else {
                        format!("   {}", g.name)
                    };
                    if ui.button(text).clicked() {
                        self.active_group_id = g.id;
                        ui.close_menu();
                    }
                }
            });

            // 4. Routing Menu (Classic Throne 1.2.0)
            ui.menu_button("Routing ▾", |ui| {
                let current_route_id = self.settings.active_route_id.unwrap_or(1);
                for r in &self.routes {
                    let text = if r.id == current_route_id {
                        format!("✓ {}", r.name)
                    } else {
                        format!("   {}", r.name)
                    };
                    if ui.button(text).clicked() {
                        let _ = self.cmd_tx.send(GuiCommand::SetRouteProfile(r.id));
                        ui.close_menu();
                    }
                }
                if self.routes.is_empty() {
                    ui.label("• Default");
                    ui.label("• Roblox ve Discord");
                }
                ui.separator();
                if ui.button("⚙ Yönlendirme Ayarları (Routing Settings)...").clicked() {
                    self.show_routing_dialog = true;
                    ui.close_menu();
                }
            });

            // 5. Tools Menu
            ui.menu_button("Tools ▾", |ui| {
                if ui.button("⏱ Tüm Düğümleri Test Et (Test All)").clicked() {
                    for p in &self.profiles {
                        let _ = self.cmd_tx.send(GuiCommand::TestProfile(p.id));
                    }
                    ui.close_menu();
                }
                if ui.button("🔄 Abonelikleri Güncelle").clicked() {
                    let _ = self.cmd_tx.send(GuiCommand::UpdateSubscription(self.active_group_id));
                    ui.close_menu();
                }
            });

            ui.add_space(4.0);

            // Circular Start / Stop Button (Exact match to old Throne 1.2.0 GUI)
            let is_running = self.status.running;
            let start_btn_resp = Self::render_start_stop_button(ui, is_running);
            if start_btn_resp.clicked() {
                if is_running {
                    let _ = self.cmd_tx.send(GuiCommand::Disconnect);
                } else if let Some(pid) = self.selected_profile_id {
                    let _ = self.cmd_tx.send(GuiCommand::Connect(pid));
                } else if !self.profiles.is_empty() {
                    let _ = self.cmd_tx.send(GuiCommand::Connect(self.profiles[0].id));
                }
            }

            ui.add_space(6.0);

            // Modes Checkboxes (Stacked side-by-side or vertical layout)
            ui.vertical(|ui| {
                let mut tun_val = self.status.tun_enabled;
                if ui.checkbox(&mut tun_val, "Tun Mode").changed() {
                    let _ = self.cmd_tx.send(GuiCommand::ToggleTun(tun_val));
                }

                let mut sysproxy_val = self.status.system_proxy;
                if ui.checkbox(&mut sysproxy_val, "System Proxy").changed() {
                    let _ = self.cmd_tx.send(GuiCommand::ToggleSystemProxy(sysproxy_val));
                }
            });

            // Far Right: Search filter
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("🔍 Filtrele...")
                        .desired_width(130.0),
                );
            });
        });
    }

    fn render_group_tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 0.0);

            for g in &self.groups {
                let is_active = g.id == self.active_group_id;
                let text = RichText::new(&g.name).color(Color32::WHITE).strong();

                let fill = if is_active {
                    Color32::from_rgb(43, 114, 214) // Blue pill
                } else {
                    Color32::from_rgb(32, 36, 44)
                };

                let btn = Button::new(text)
                    .fill(fill)
                    .corner_radius(egui::CornerRadius::same(4))
                    .min_size(vec2(65.0, 24.0));

                if ui.add(btn).clicked() {
                    self.active_group_id = g.id;
                }
            }

            // Funnel / Filter Icon aligned to right (Matching old Throne screenshot)
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let filter_btn = Button::new(RichText::new(" 🔻 ").color(Color32::from_rgb(88, 166, 255)))
                    .fill(Color32::from_rgb(30, 42, 60))
                    .corner_radius(egui::CornerRadius::same(4))
                    .min_size(vec2(28.0, 22.0));
                if ui.add(filter_btn).clicked() {
                    self.search_query.clear();
                }
            });
        });
    }

    fn render_profiles_table(&mut self, ui: &mut Ui) {
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
        let row_height = text_height + 11.0;

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
            .striped(false)
            .resizable(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::exact(36.0))              // ✓ or #
            .column(Column::initial(80.0).at_least(60.0))   // Type
            .column(Column::initial(180.0).at_least(110.0)) // Address
            .column(Column::remainder().at_least(180.0))    // Name
            .column(Column::initial(90.0).at_least(70.0))   // Test Result
            .column(Column::initial(180.0).at_least(140.0)) // Traffic
            .header(24.0, |mut header| {
                header.col(|ui| { ui.strong("✓"); });
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
                        "{} ↑ {} ↓",
                        Self::format_bytes(p.traffic_up),
                        Self::format_bytes(p.traffic_dl)
                    );

                    let latency_str = if p.latency > 0 {
                        format!("{} ms", p.latency)
                    } else {
                        String::new()
                    };

                    // In old Throne: selected row has solid purple background (#7c4dff)
                    let purple_bg = Color32::from_rgb(124, 77, 255);

                    body.row(row_height, |mut row| {
                        // 1. Checkmark ✓ or Index
                        row.col(|ui| {
                            if is_selected {
                                ui.painter().rect_filled(ui.max_rect(), 0.0, purple_bg);
                            }
                            let label = if is_active_profile {
                                RichText::new("✓").color(Color32::WHITE).strong()
                            } else {
                                RichText::new(format!("{}", idx + 1)).color(if is_selected { Color32::WHITE } else { Color32::from_rgb(160, 168, 178) })
                            };
                            let resp = ui.selectable_label(false, label);
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
                            if is_selected {
                                ui.painter().rect_filled(ui.max_rect(), 0.0, purple_bg);
                            }
                            let text = RichText::new(&p.profile_type).color(Color32::WHITE);
                            let resp = ui.selectable_label(false, text);
                            if resp.clicked() {
                                self.selected_profile_id = Some(p.id);
                            }
                        });

                        // 3. Address
                        row.col(|ui| {
                            if is_selected {
                                ui.painter().rect_filled(ui.max_rect(), 0.0, purple_bg);
                            }
                            let text = RichText::new(&addr).color(Color32::WHITE);
                            let resp = ui.selectable_label(false, text);
                            if resp.clicked() {
                                self.selected_profile_id = Some(p.id);
                            }
                        });

                        // 4. Name
                        row.col(|ui| {
                            if is_selected {
                                ui.painter().rect_filled(ui.max_rect(), 0.0, purple_bg);
                            }
                            let text = RichText::new(&p.name).color(Color32::WHITE);
                            let resp = ui.selectable_label(false, text);
                            if resp.clicked() {
                                self.selected_profile_id = Some(p.id);
                            }
                            if resp.double_clicked() {
                                self.selected_profile_id = Some(p.id);
                                let _ = self.cmd_tx.send(GuiCommand::Connect(p.id));
                            }
                            resp.context_menu(|ui| {
                                if ui.button("⚡ Bağlan (Connect)").clicked() {
                                    let _ = self.cmd_tx.send(GuiCommand::Connect(p.id));
                                    ui.close_menu();
                                }
                                if ui.button("⏱ Test Et (Latency Test)").clicked() {
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
                            if is_selected {
                                ui.painter().rect_filled(ui.max_rect(), 0.0, purple_bg);
                            }
                            let text = RichText::new(&latency_str).color(Color32::WHITE);
                            let resp = ui.selectable_label(false, text);
                            if resp.clicked() {
                                self.selected_profile_id = Some(p.id);
                            }
                        });

                        // 6. Traffic
                        row.col(|ui| {
                            if is_selected {
                                ui.painter().rect_filled(ui.max_rect(), 0.0, purple_bg);
                            }
                            let text = RichText::new(&traffic_str).color(Color32::WHITE);
                            let resp = ui.selectable_label(false, text);
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
            ui.spacing_mut().item_spacing = vec2(3.0, 0.0);

            let tabs = [
                (BottomTab::Logs, "Logs"),
                (BottomTab::Connections, "Connections"),
                (BottomTab::TrafficGraph, "Traffic Graph"),
            ];

            for (tab, title) in tabs {
                let is_active = self.active_bottom_tab == tab;
                let text = RichText::new(title).color(if is_active {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(140, 148, 158)
                });

                let fill = if is_active {
                    Color32::from_rgb(36, 40, 48)
                } else {
                    Color32::from_rgb(22, 24, 28)
                };

                let btn = Button::new(text)
                    .fill(fill)
                    .corner_radius(egui::CornerRadius::same(3))
                    .min_size(vec2(76.0, 22.0));

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

        ui.add_space(2.0);

        match self.active_bottom_tab {
            BottomTab::Logs => {
                egui::Frame::NONE
                    .fill(Color32::from_rgb(16, 16, 16))
                    .inner_margin(6.0)
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .stick_to_bottom(self.auto_scroll_logs)
                            .show(ui, |ui| {
                                ui.style_mut().override_font_id = Some(egui::FontId::monospace(12.0));
                                for line in &self.log_lines {
                                    let color = if line.contains("[ERROR]") || line.contains("FATAL") {
                                        Color32::from_rgb(248, 81, 73)
                                    } else if line.contains("[SUCCESS]") || line.contains("Core Has Successfully") {
                                        Color32::from_rgb(63, 185, 80)
                                    } else if line.contains("[INIT]") || line.contains("[CONNECT]") || line.contains("INFO") {
                                        Color32::from_rgb(88, 166, 255)
                                    } else if line.contains("WARN") {
                                        Color32::from_rgb(227, 179, 65)
                                    } else {
                                        Color32::from_rgb(201, 209, 217)
                                    };
                                    ui.colored_label(color, line);
                                }
                            });
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
                        ui.heading("Aktif Tünel ve Vekil Bağlantıları");
                        ui.label(format!("Inbound Dinleyici: 127.0.0.1:{}", self.status.inbound_port));
                        ui.label(format!("DPI Atlama: 127.0.0.1:1080 (ciadpi - {})", self.settings.byedpi_preset));
                        ui.label(format!("TUN Durumu: {}", if self.status.tun_enabled { "Açık (throne-tun)" } else { "Kapalı" }));
                        ui.label(format!("Sistem Vekili: {}", if self.status.system_proxy { "Açık (127.0.0.1:2080)" } else { "Kapalı" }));
                    } else {
                        ui.label("Bağlantı aktif değil.");
                    }
                });
            }
        }
    }

    fn render_status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(16.0, 0.0);

            // Left: [Default] Profile Name + location
            let active_profile_name = self
                .status
                .active_profile_id
                .and_then(|id| self.profiles.iter().find(|p| p.id == id).map(|p| p.name.clone()))
                .unwrap_or_else(|| {
                    if !self.profiles.is_empty() {
                        self.profiles[0].name.clone()
                    } else {
                        "My Config".into()
                    }
                });

            let status_prefix = if self.status.running {
                "🟢"
            } else {
                "⚪"
            };

            ui.colored_label(
                if self.status.running { Color32::from_rgb(63, 185, 80) } else { Color32::from_rgb(140, 148, 158) },
                format!("{} [Default] {}  🇩🇪 Germany", status_prefix, active_profile_name),
            );

            ui.separator();

            // Center: Mixed inbound address & port
            ui.colored_label(
                Color32::from_rgb(140, 148, 158),
                format!("Mixed: {}:{}", self.status.inbound_address, self.status.inbound_port),
            );

            // Right: Speeds matching old Throne status bar
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(format!(
                    "Proxy: {} ↑  {} ↓   Direct: 0.00 B↑ 0.00 B↓",
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
        egui::Window::new("Ayarlar (Preferences)")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size(vec2(580.0, 480.0))
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                // Tab bar matching old Throne
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                    let tabs = [
                        (SettingsTab::Common, "Genel (Common)"),
                        (SettingsTab::Core, "Çekirdek (Core)"),
                        (SettingsTab::Dns, "DNS"),
                        (SettingsTab::Tun, "TUN / VPN"),
                        (SettingsTab::Routing, "Yönlendirme"),
                        (SettingsTab::ByeDpi, "ByeDPI (Vodafone)"),
                        (SettingsTab::SystemProxy, "Sistem Vekili"),
                    ];

                    for (tab, name) in tabs {
                        let is_active = self.active_settings_tab == tab;
                        let text = RichText::new(name).color(if is_active { Color32::WHITE } else { Color32::from_rgb(150, 160, 172) });
                        let fill = if is_active { Color32::from_rgb(43, 114, 214) } else { Color32::from_rgb(26, 30, 38) };
                        if ui.add(Button::new(text).fill(fill).corner_radius(egui::CornerRadius::same(3))).clicked() {
                            self.active_settings_tab = tab;
                        }
                    }
                });

                ui.separator();

                ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    match self.active_settings_tab {
                        SettingsTab::Common => {
                            ui.heading("Inbound & Genel Ayarlar");
                            ui.add_space(6.0);
                            egui::Grid::new("common_grid").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
                                ui.label("Inbound Dinleme Portu:");
                                ui.add(egui::DragValue::new(&mut self.settings.inbound_port).range(1024..=65535));
                                ui.end_row();

                                ui.label("Inbound Dinleme Adresi:");
                                ui.text_edit_singleline(&mut self.settings.inbound_address);
                                ui.end_row();

                                ui.label("Yerel Ağa İzin Ver (Allow LAN):");
                                let mut allow_lan = self.settings.inbound_address == "0.0.0.0" || self.settings.inbound_address == "::";
                                if ui.checkbox(&mut allow_lan, "Tüm arayüzlerde dinle (0.0.0.0)").changed() {
                                    self.settings.inbound_address = if allow_lan { "0.0.0.0".into() } else { "127.0.0.1".into() };
                                }
                                ui.end_row();

                                ui.label("Uzak API Etkinleştir:");
                                ui.checkbox(&mut self.settings.remote_api_enable, "9095 HTTP/REST API");
                                ui.end_row();

                                ui.label("API Portu:");
                                ui.add(egui::DragValue::new(&mut self.settings.remote_api_port).range(1024..=65535));
                                ui.end_row();
                            });
                        }
                        SettingsTab::Core => {
                            ui.heading("Çekirdek İşlemleri & Seçenekler");
                            ui.add_space(6.0);
                            egui::Grid::new("core_grid").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
                                ui.label("Çekirdek Program Yolu:");
                                ui.text_edit_singleline(&mut self.settings.core_path);
                                ui.end_row();

                                ui.label("Günlük Ayrıntı Düzeyi (Log Level):");
                                egui::ComboBox::from_id_salt("core_log_lvl")
                                    .selected_text(&self.settings.log_level)
                                    .show_ui(ui, |ui| {
                                        for lvl in &["trace", "debug", "info", "warn", "error"] {
                                            ui.selectable_value(&mut self.settings.log_level, lvl.to_string(), *lvl);
                                        }
                                    });
                                ui.end_row();

                                ui.label("TCP MUX (Çoklama):");
                                ui.checkbox(&mut self.settings.mux_enabled, "Bağlantıları birleştir");
                                ui.end_row();
                            });
                        }
                        SettingsTab::Dns => {
                            ui.heading("DNS & Alan Adı Çözümleme Ayarları");
                            ui.add_space(6.0);
                            egui::Grid::new("dns_grid").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
                                ui.label("Uzak DNS (Remote DNS - DoH / Proxy):");
                                ui.text_edit_singleline(&mut self.settings.remote_dns);
                                ui.end_row();

                                ui.label("Doğrudan DNS (Direct DNS):");
                                ui.text_edit_singleline(&mut self.settings.direct_dns);
                                ui.end_row();

                                ui.label("Sahte DNS (FakeDNS):");
                                ui.checkbox(&mut self.settings.fake_dns, "FakeDNS Havuzunu Kullan");
                                ui.end_row();

                                ui.label("Ön Tanımlı Alan Adları (Predefined):");
                                ui.checkbox(&mut self.settings.dns_predefined_enable, "Discord & Roblox doğrudan IP çözümlemesi");
                                ui.end_row();
                            });

                            ui.add_space(8.0);
                            ui.label("Ön Tanımlı DNS Eşlemeleri (Her satıra bir 'IP AlanAdı' formatı):");
                            ui.add(
                                egui::TextEdit::multiline(&mut self.dns_predefined_text)
                                    .desired_rows(6)
                                    .desired_width(ui.available_width() - 10.0),
                            );
                        }
                        SettingsTab::Tun => {
                            ui.heading("TUN / VPN Sanal Ağ Arayüzü");
                            ui.add_space(6.0);
                            egui::Grid::new("tun_grid").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
                                ui.label("TUN Modu:");
                                ui.checkbox(&mut self.settings.enable_tun, "TUN Sanal Arayüzünü Etkinleştir");
                                ui.end_row();

                                ui.label("Arayüz Adı (Interface Name):");
                                ui.text_edit_singleline(&mut self.settings.tun_interface_name);
                                ui.end_row();

                                ui.label("Ağ Yığını (Network Stack):");
                                egui::ComboBox::from_id_salt("tun_stack_combo")
                                    .selected_text(&self.settings.tun_stack)
                                    .show_ui(ui, |ui| {
                                        for st in &["system", "gvisor", "mixed"] {
                                            ui.selectable_value(&mut self.settings.tun_stack, st.to_string(), *st);
                                        }
                                    });
                                ui.end_row();

                                ui.label("Sıkı Yönlendirme (Strict Route):");
                                ui.checkbox(&mut self.settings.tun_strict_route, "Tüm trafiği zorla yönlendir");
                                ui.end_row();
                            });
                        }
                        SettingsTab::Routing => {
                            ui.heading("Yönlendirme & Kural Profili");
                            ui.add_space(6.0);
                            let cur_rid = self.settings.active_route_id.unwrap_or(1);
                            let cur_name = self.routes.iter().find(|r| r.id == cur_rid).map(|r| r.name.as_str()).unwrap_or("Default / Roblox ve Discord");

                            ui.horizontal(|ui| {
                                ui.label("Aktif Kural Profili:");
                                egui::ComboBox::from_id_salt("route_profile_combo")
                                    .selected_text(cur_name)
                                    .show_ui(ui, |ui| {
                                        for r in &self.routes {
                                            if ui.selectable_label(r.id == cur_rid, &r.name).clicked() {
                                                self.settings.active_route_id = Some(r.id);
                                            }
                                        }
                                    });
                            });

                            ui.add_space(8.0);
                            if ui.button("⚙ Yönlendirme Yöneticisini Aç (Open Route Manager)...").clicked() {
                                self.show_routing_dialog = true;
                            }

                            ui.add_space(8.0);
                            ui.label("Dahili Kurallar:");
                            ui.label("• ciadpi (ByeDPI) -> Direct (Döngü koruması)");
                            ui.label("• Port 53 / Protocol DNS -> Hijack-DNS (Şifreli DoH)");
                            ui.label("• Özel IP Aralıkları (Private RFC1918) -> Direct");
                            ui.label("• Discord & Roblox (updates.discord.com, roblox.com) -> Proxy (ByeDPI)");
                        }
                        SettingsTab::ByeDpi => {
                            ui.heading("ByeDPI (ciadpi) Vodafone DPI Atlama Denetimi");
                            ui.add_space(6.0);
                            egui::Grid::new("byedpi_grid").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
                                ui.label("Otomatik Başlat:");
                                ui.checkbox(&mut self.settings.byedpi_enable, "Profile bağlanınca ByeDPI'ı otomatik başlat");
                                ui.end_row();

                                ui.label("ByeDPI Dosya Yolu:");
                                ui.text_edit_singleline(&mut self.settings.byedpi_path);
                                ui.end_row();

                                ui.label("DPI Atlama Önayarı (Preset):");
                                egui::ComboBox::from_id_salt("byedpi_preset_combo")
                                    .selected_text(&self.settings.byedpi_preset)
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_value(&mut self.settings.byedpi_preset, "vodafone".into(), "Vodafone DPI Bypass (-s1 -At -d2 -f-1 -r1+s -An)").clicked() {
                                            self.settings.byedpi_args = "-s1 -At -d2 -f-1 -r1+s -An".into();
                                        }
                                        if ui.selectable_value(&mut self.settings.byedpi_preset, "standard".into(), "Standard Bypass (-s1 -q1 -Y -Ar)").clicked() {
                                            self.settings.byedpi_args = "-s1 -q1 -Y -Ar".into();
                                        }
                                        if ui.selectable_value(&mut self.settings.byedpi_preset, "custom".into(), "Özel (Custom)").clicked() {}
                                    });
                                ui.end_row();

                                ui.label("Komut Satırı Parametreleri:");
                                ui.text_edit_singleline(&mut self.settings.byedpi_args);
                                ui.end_row();
                            });
                        }
                        SettingsTab::SystemProxy => {
                            ui.heading("Sistem Vekil Sunucusu (System Proxy)");
                            ui.add_space(6.0);
                            egui::Grid::new("sysproxy_grid").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
                                ui.label("Sistem Vekili Etkin:");
                                ui.checkbox(&mut self.settings.system_proxy, "HTTP/HTTPS/SOCKS sistem vekili");
                                ui.end_row();
                            });

                            ui.add_space(8.0);
                            ui.label("Desteklenen Masaüstü ve Ortamlar:");
                            ui.label("• KDE Plasma 6 & 5 (kioslaverc ve KIO Scheduler)");
                            ui.label("• GNOME (gsettings org.gnome.system.proxy)");
                            ui.label("• Electron / Discord (systemd user environment http_proxy/all_proxy)");
                        }
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("💾 Kaydet (Save)").clicked() {
                        // Parse updated predefined rules text
                        self.settings.dns_predefined_rules = self
                            .dns_predefined_text
                            .lines()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();

                        let _ = self.cmd_tx.send(GuiCommand::SaveSettings(Box::new(self.settings.clone())));
                        self.show_settings_dialog = false;
                    }
                    if ui.button("❌ İptal (Cancel)").clicked() {
                        self.show_settings_dialog = false;
                    }
                });
            });

        if !open {
            self.show_settings_dialog = false;
        }
    }

    fn render_routing_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_routing_dialog {
            return;
        }

        let mut open = true;
        egui::Window::new("⚙ Yönlendirme ve Kural Yöneticisi (Routing Settings)")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size(vec2(860.0, 560.0))
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left column: Profile selector and management
                    ui.vertical(|ui| {
                        ui.set_width(220.0);
                        ui.heading("Profiller");
                        ui.add_space(4.0);

                        ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                            let active_id = self.settings.active_route_id.unwrap_or(1);
                            for p in &self.routes {
                                let is_selected = self.routing_selected_profile_id == Some(p.id);
                                let is_active = p.id == active_id;
                                let title = if is_active {
                                    format!("★ {} (Aktif)", p.name)
                                } else {
                                    p.name.clone()
                                };

                                let resp = ui.selectable_label(is_selected, title);
                                if resp.clicked() {
                                    self.routing_selected_profile_id = Some(p.id);
                                    self.editing_route_profile = Some(p.clone());
                                    let _ = self.cmd_tx.send(GuiCommand::LoadRouteRules(p.id));
                                }
                            }
                        });

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("➕ Yeni").clicked() {
                                self.new_profile_name = "Yeni Profil".into();
                                self.show_new_profile_modal = true;
                            }
                            if ui.button("📋 Klonla").clicked()
                                && let Some(ref cur) = self.editing_route_profile
                            {
                                let clone_name = format!("{} (Kopya)", cur.name);
                                let _ = self.cmd_tx.send(GuiCommand::CreateRouteProfile(clone_name));
                            }
                            if self.routes.len() > 1
                                && ui.button("🗑️ Sil").clicked()
                                && let Some(id) = self.routing_selected_profile_id
                            {
                                let _ = self.cmd_tx.send(GuiCommand::DeleteRouteProfile(id));
                                self.routing_selected_profile_id = None;
                                self.editing_route_profile = None;
                                self.editing_route_rules.clear();
                            }
                        });
                    });

                    ui.separator();

                    // Right column: Selected Profile Editor & Rules Table
                    ui.vertical(|ui| {
                        if let Some(ref mut prof) = self.editing_route_profile {
                            ui.horizontal(|ui| {
                                ui.label("Profil Adı:");
                                ui.text_edit_singleline(&mut prof.name);

                                ui.label("Varsayılan Çıkış:");
                                egui::ComboBox::from_id_salt("default_outbound_combo")
                                    .selected_text(match prof.default_outbound_id {
                                        -1 => "Proxy (ByeDPI)",
                                        -2 => "Direct (Bypass)",
                                        -3 => "Block (Engelle)",
                                        _ => "Direct (Bypass)",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut prof.default_outbound_id, -1, "Proxy (ByeDPI)");
                                        ui.selectable_value(&mut prof.default_outbound_id, -2, "Direct (Bypass)");
                                        ui.selectable_value(&mut prof.default_outbound_id, -3, "Block (Engelle)");
                                    });

                                let is_active = self.settings.active_route_id == Some(prof.id);
                                if !is_active {
                                    if ui.button("★ Bu Profili Aktif Yap").clicked() {
                                        let _ = self.cmd_tx.send(GuiCommand::SetRouteProfile(prof.id));
                                    }
                                } else {
                                    ui.colored_label(Color32::from_rgb(76, 175, 80), "✓ Aktif Profil");
                                }
                            });

                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.heading(format!("Kurallar ({})", self.editing_route_rules.len()));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.button("➕ Kural Ekle").clicked() {
                                        self.editing_rule_index = None;
                                        self.rule_draft = RouteRule {
                                            name: "Yeni Kural".into(),
                                            target: "proxy".into(),
                                            ..Default::default()
                                        };
                                        self.rule_domains_text.clear();
                                        self.rule_suffixes_text.clear();
                                        self.rule_processes_text.clear();
                                        self.rule_cidrs_text.clear();
                                        self.show_rule_editor = true;
                                    }
                                });
                            });

                            ui.add_space(4.0);

                            ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                                let mut to_delete = None;
                                let mut to_move_up = None;
                                let mut to_move_down = None;
                                let mut to_edit = None;

                                egui::Grid::new("rules_table_grid")
                                    .striped(true)
                                    .num_columns(5)
                                    .spacing([12.0, 8.0])
                                    .show(ui, |ui| {
                                        ui.strong("#");
                                        ui.strong("Kural Adı");
                                        ui.strong("Çıkış");
                                        ui.strong("Hedefler / Koşullar");
                                        ui.strong("İşlemler");
                                        ui.end_row();

                                        for (idx, r) in self.editing_route_rules.iter().enumerate() {
                                            ui.label(format!("{}", idx + 1));
                                            ui.label(&r.name);

                                            let target_text = match r.target.as_str() {
                                                "proxy" => RichText::new("Proxy").color(Color32::from_rgb(100, 180, 255)),
                                                "direct" => RichText::new("Direct").color(Color32::from_rgb(120, 220, 120)),
                                                "block" => RichText::new("Block").color(Color32::from_rgb(255, 100, 100)),
                                                _ => RichText::new(&r.target),
                                            };
                                            ui.label(target_text);

                                            // Conditions summary
                                            let mut conditions = Vec::new();
                                            if !r.domain_suffixes.is_empty() {
                                                conditions.push(format!("Alan: {}", r.domain_suffixes.join(", ")));
                                            }
                                            if !r.domains.is_empty() {
                                                conditions.push(format!("Tam Alan: {}", r.domains.join(", ")));
                                            }
                                            if !r.process_names.is_empty() {
                                                conditions.push(format!("İşlem: {}", r.process_names.join(", ")));
                                            }
                                            if !r.ip_cidrs.is_empty() {
                                                conditions.push(format!("IP: {}", r.ip_cidrs.join(", ")));
                                            }
                                            let cond_str = if conditions.is_empty() {
                                                "(Tüm Trafik)".to_string()
                                            } else {
                                                conditions.join(" | ")
                                            };
                                            ui.label(cond_str);

                                            ui.horizontal(|ui| {
                                                if ui.small_button("✏").clicked() {
                                                    to_edit = Some(idx);
                                                }
                                                if idx > 0 && ui.small_button("⬆").clicked() {
                                                    to_move_up = Some(idx);
                                                }
                                                if idx + 1 < self.editing_route_rules.len() && ui.small_button("⬇").clicked() {
                                                    to_move_down = Some(idx);
                                                }
                                                if ui.small_button("🗑").clicked() {
                                                    to_delete = Some(idx);
                                                }
                                            });
                                            ui.end_row();
                                        }
                                    });

                                if let Some(idx) = to_edit {
                                    let r = &self.editing_route_rules[idx];
                                    self.editing_rule_index = Some(idx);
                                    self.rule_draft = r.clone();
                                    self.rule_domains_text = r.domains.join("\n");
                                    self.rule_suffixes_text = r.domain_suffixes.join("\n");
                                    self.rule_processes_text = r.process_names.join("\n");
                                    self.rule_cidrs_text = r.ip_cidrs.join("\n");
                                    self.show_rule_editor = true;
                                }
                                if let Some(idx) = to_move_up {
                                    self.editing_route_rules.swap(idx, idx - 1);
                                }
                                if let Some(idx) = to_move_down {
                                    self.editing_route_rules.swap(idx, idx + 1);
                                }
                                if let Some(idx) = to_delete {
                                    self.editing_route_rules.remove(idx);
                                }
                            });

                            ui.separator();

                            ui.horizontal(|ui| {
                                if ui.button("💾 Değişiklikleri Kaydet & Uygula").clicked() {
                                    let prof_clone = prof.clone();
                                    let rules_clone = self.editing_route_rules.clone();
                                    let _ = self.cmd_tx.send(GuiCommand::SaveRouteProfile(prof_clone, rules_clone));
                                }
                                if ui.button("Kapat").clicked() {
                                    self.show_routing_dialog = false;
                                }
                            });
                        } else {
                            ui.label("Düzenlemek için soldaki listeden bir profil seçin.");
                        }
                    });
                });
            });

        if !open {
            self.show_routing_dialog = false;
        }
    }

    fn render_rule_editor_modal(&mut self, ctx: &egui::Context) {
        if !self.show_rule_editor {
            return;
        }

        let mut open = true;
        egui::Window::new("Kural Düzenle (Edit Rule)")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_size(vec2(520.0, 440.0))
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Kural Adı:");
                    ui.text_edit_singleline(&mut self.rule_draft.name);

                    ui.label("Çıkış (Outbound):");
                    egui::ComboBox::from_id_salt("draft_outbound")
                        .selected_text(match self.rule_draft.target.as_str() {
                            "proxy" => "Proxy (ByeDPI)",
                            "direct" => "Direct (Bypass)",
                            "block" => "Block (Engelle)",
                            _ => &self.rule_draft.target,
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.rule_draft.target, "proxy".into(), "Proxy (ByeDPI)");
                            ui.selectable_value(&mut self.rule_draft.target, "direct".into(), "Direct (Bypass)");
                            ui.selectable_value(&mut self.rule_draft.target, "block".into(), "Block (Engelle)");
                        });
                });

                ui.separator();

                ui.label("Alan Adı Sonekleri (Domain Suffix - virgülle veya satır satır ayırın):");
                ui.add(egui::TextEdit::multiline(&mut self.rule_suffixes_text).desired_rows(3).desired_width(480.0));

                ui.label("İşlem Adları (Process Name - örn: Discord, updater_bootstrap, sober):");
                ui.add(egui::TextEdit::multiline(&mut self.rule_processes_text).desired_rows(3).desired_width(480.0));

                ui.label("IP CIDR Aralıkları (örn: 1.1.1.1/32, 192.168.0.0/16):");
                ui.add(egui::TextEdit::multiline(&mut self.rule_cidrs_text).desired_rows(2).desired_width(480.0));

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("✓ Tamam").clicked() {
                        let parse_lines = |text: &str| -> Vec<String> {
                            text.lines()
                                .flat_map(|l| l.split(','))
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect()
                        };

                        self.rule_draft.domain_suffixes = parse_lines(&self.rule_suffixes_text);
                        self.rule_draft.process_names = parse_lines(&self.rule_processes_text);
                        self.rule_draft.ip_cidrs = parse_lines(&self.rule_cidrs_text);

                        if let Some(idx) = self.editing_rule_index {
                            if idx < self.editing_route_rules.len() {
                                self.editing_route_rules[idx] = self.rule_draft.clone();
                            }
                        } else {
                            self.editing_route_rules.push(self.rule_draft.clone());
                        }

                        self.show_rule_editor = false;
                    }
                    if ui.button("İptal").clicked() {
                        self.show_rule_editor = false;
                    }
                });
            });

        if !open {
            self.show_rule_editor = false;
        }
    }

    fn render_new_profile_modal(&mut self, ctx: &egui::Context) {
        if !self.show_new_profile_modal {
            return;
        }

        let mut open = true;
        egui::Window::new("Yeni Yönlendirme Profili")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("Profil Adı:");
                ui.text_edit_singleline(&mut self.new_profile_name);

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Oluştur").clicked() {
                        let name = self.new_profile_name.trim().to_string();
                        if !name.is_empty() {
                            let _ = self.cmd_tx.send(GuiCommand::CreateRouteProfile(name));
                            self.show_new_profile_modal = false;
                        }
                    }
                    if ui.button("İptal").clicked() {
                        self.show_new_profile_modal = false;
                    }
                });
            });

        if !open {
            self.show_new_profile_modal = false;
        }
    }
}

impl eframe::App for ThroneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();

        // Top Panel: Toolbar & Groups row
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
                self.render_top_bar(ui);
                ui.add_space(6.0);
                self.render_group_tabs(ui);
            });

        // Bottom Panel: Status bar matching old Throne
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(6.0))
            .show(ctx, |ui| {
                self.render_status_bar(ui);
            });

        // Bottom Split Panel: Logs, Connections, Traffic Graph
        egui::TopBottomPanel::bottom("bottom_tabs_panel")
            .resizable(true)
            .default_height(220.0)
            .min_height(120.0)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(6.0))
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
        self.render_routing_dialog(ctx);
        self.render_rule_editor_modal(ctx);
        self.render_new_profile_modal(ctx);

        // Keep UI reactive
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}
