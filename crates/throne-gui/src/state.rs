use throne_core::{Group, Profile, Settings};

#[derive(Debug)]
pub enum GuiCommand {
    Connect(i64),
    Disconnect,
    ToggleTun(bool),
    ToggleSystemProxy(bool),
    ToggleByeDpi(bool),
    SetByeDpiPreset(String),
    TestProfile(i64),
    DeleteProfile(i64),
    ImportSource(String, i64),
    UpdateSubscription(i64),
    SaveSettings(Box<Settings>),
    RefreshData,
}

#[derive(Debug, Clone)]
pub struct StatusState {
    pub running: bool,
    pub byedpi_running: bool,
    pub active_profile_id: Option<i64>,
    pub tun_enabled: bool,
    pub system_proxy: bool,
    pub inbound_port: u16,
    pub inbound_address: String,
}

#[derive(Debug, Clone)]
pub enum GuiEvent {
    StatusUpdated(StatusState),
    LogAppended(String),
    ProfilesUpdated(Vec<Profile>),
    GroupsUpdated(Vec<Group>),
    SettingsUpdated(Box<Settings>),
    LatencyUpdated(i64, Option<i32>),
    SpeedSample { up_bytes: f64, down_bytes: f64 },
}
