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

            // KDE Plasma 6 / 5 (kwriteconfig)
            let http_url = format!("http://{}:{}", host, port);
            let socks_url = format!("socks://{}:{}", host, port);
            for tool in &["kwriteconfig6", "kwriteconfig5"] {
                let _ = Command::new(tool)
                    .args(["--file", "kioslaverc", "--group", "Proxy Settings", "--key", "ProxyType", "1"])
                    .status();
                let _ = Command::new(tool)
                    .args(["--file", "kioslaverc", "--group", "Proxy Settings", "--key", "httpProxy", &http_url])
                    .status();
                let _ = Command::new(tool)
                    .args(["--file", "kioslaverc", "--group", "Proxy Settings", "--key", "httpsProxy", &http_url])
                    .status();
                let _ = Command::new(tool)
                    .args(["--file", "kioslaverc", "--group", "Proxy Settings", "--key", "socksProxy", &socks_url])
                    .status();
            }
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

            for tool in &["kwriteconfig6", "kwriteconfig5"] {
                let _ = Command::new(tool)
                    .args(["--file", "kioslaverc", "--group", "Proxy Settings", "--key", "ProxyType", "0"])
                    .status();
            }
        }

        Ok(())
    }
}
