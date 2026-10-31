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

            // Notify KDE Plasma KIO scheduler
            let _ = Command::new("dbus-send")
                .args(["--type=signal", "/KIO/Scheduler", "org.kde.KIO.Scheduler.reparseSlaveConfiguration", "string:"])
                .status();

            // Systemd user environment (ensures all Electron apps like Discord and terminal tools inherit proxy)
            let http_env = format!("http_proxy={}", http_url);
            let https_env = format!("https_proxy={}", http_url);
            let all_env = format!("all_proxy=socks5://{}:{}", host, port);
            let http_upper = format!("HTTP_PROXY={}", http_url);
            let https_upper = format!("HTTPS_PROXY={}", http_url);
            let all_upper = format!("ALL_PROXY=socks5://{}:{}", host, port);
            let _ = Command::new("systemctl")
                .args(["--user", "set-environment", &http_env, &https_env, &all_env, &http_upper, &https_upper, &all_upper])
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

            for tool in &["kwriteconfig6", "kwriteconfig5"] {
                let _ = Command::new(tool)
                    .args(["--file", "kioslaverc", "--group", "Proxy Settings", "--key", "ProxyType", "0"])
                    .status();
            }

            // Notify KDE Plasma KIO scheduler
            let _ = Command::new("dbus-send")
                .args(["--type=signal", "/KIO/Scheduler", "org.kde.KIO.Scheduler.reparseSlaveConfiguration", "string:"])
                .status();

            // Clear systemd user environment
            let _ = Command::new("systemctl")
                .args(["--user", "unset-environment", "http_proxy", "https_proxy", "all_proxy", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY"])
                .status();
        }

        Ok(())
    }
}
