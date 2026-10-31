use std::process::Command;
use tracing::info;

pub struct SystemProxyManager;

impl SystemProxyManager {
    pub fn enable(host: &str, port: u16) -> anyhow::Result<()> {
        info!("Enabling system proxy: {}:{}", host, port);

        #[cfg(target_os = "linux")]
        {
            // GNOME gsettings
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy.http", "host", host])
                .status();
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy.http", "port", &port.to_string()])
                .status();
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy.https", "host", host])
                .status();
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy.https", "port", &port.to_string()])
                .status();
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy.socks", "host", host])
                .status();
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy.socks", "port", &port.to_string()])
                .status();
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy", "mode", "manual"])
                .status();
        }

        Ok(())
    }

    pub fn disable() -> anyhow::Result<()> {
        info!("Disabling system proxy");

        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("gsettings")
                .args(["set", "org.gnome.system.proxy", "mode", "none"])
                .status();
        }

        Ok(())
    }
}
