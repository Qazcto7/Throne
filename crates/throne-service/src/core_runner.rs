use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::info;

pub fn resolve_core_bin(custom: Option<&Path>) -> PathBuf {
    if let Some(p) = custom
        && p.exists()
    {
        return p.to_path_buf();
    }

    // Check environment variables
    if let Ok(env_path) = std::env::var("SING_BOX_BIN") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return p;
        }
    }
    if let Ok(env_path) = std::env::var("CORE_PATH") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return p;
        }
    }

    // Check beside current executable
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let p = parent.join("sing-box");
        if p.exists() {
            return p;
        }
    }

    // Check user home known locations
    if let Some(home) = dirs::home_dir() {
        let p1 = home.join(".local").join("bin").join("sing-box");
        if p1.exists() {
            return p1;
        }
        let p2 = home.join("byedpi_throne_linux").join("Throne").join("sing-box");
        if p2.exists() {
            return p2;
        }
        let p3 = home.join("byedpi_throne_linux").join("sing-box");
        if p3.exists() {
            return p3;
        }
    }

    // Check current working directory
    let cwd_bin = PathBuf::from("./sing-box");
    if cwd_bin.exists() {
        return cwd_bin;
    }

    // System standard locations
    for sys in &["/usr/local/bin/sing-box", "/usr/bin/sing-box", "/opt/sing-box/sing-box"] {
        let p = PathBuf::from(sys);
        if p.exists() {
            return p;
        }
    }

    // Fall back to "sing-box" in PATH
    PathBuf::from("sing-box")
}

fn cleanup_tun_stale() {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("pkill")
            .args(["-15", "-f", "sing-box"])
            .status();

        // Wait up to 1000ms for throne-tun device to disappear from kernel
        for _ in 0..20 {
            let status = std::process::Command::new("ip")
                .args(["link", "show", "throne-tun"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            if let Ok(st) = status
                && !st.success()
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

pub struct CoreRunner {
    core_path: String,
    child: Arc<Mutex<Option<Child>>>,
    config_file: Arc<Mutex<Option<tempfile::NamedTempFile>>>,
    pid_file: PathBuf,
}

impl CoreRunner {
    pub fn new(core_path: String) -> Self {
        let resolved = resolve_core_bin(Some(Path::new(&core_path)));
        let pid_file = std::env::temp_dir().join("throne-singbox.pid");
        Self {
            core_path: resolved.to_string_lossy().to_string(),
            child: Arc::new(Mutex::new(None)),
            config_file: Arc::new(Mutex::new(None)),
            pid_file,
        }
    }

    pub async fn start(&self, config_json: &serde_json::Value) -> anyhow::Result<()> {
        self.stop().await?;
        cleanup_tun_stale();

        let core_bin = resolve_core_bin(Some(Path::new(&self.core_path)));

        // Write config to temporary file
        let mut tmp = tempfile::Builder::new()
            .prefix("throne-singbox-")
            .suffix(".json")
            .tempfile()?;

        use std::io::Write;
        tmp.write_all(serde_json::to_string_pretty(config_json)?.as_bytes())?;
        let config_path = tmp.path().to_path_buf();

        info!("Starting core proxy with binary {:?} and config {:?}", core_bin, config_path);

        let mut child = match Command::new(&core_bin)
            .arg("run")
            .arg("-c")
            .arg(&config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                anyhow::bail!("Failed to spawn core process '{}': {}", core_bin.display(), e);
            }
        };

        if let Some(pid) = child.id() {
            let _ = std::fs::write(&self.pid_file, pid.to_string());
        }

        // Check if process exited immediately on error
        tokio::time::sleep(tokio::time::Duration::from_millis(350)).await;
        if let Ok(Some(status)) = child.try_wait() {
            let mut err_msg = format!("Core process exited with status {}", status);
            if let Some(mut stderr) = child.stderr.take() {
                let mut buf = String::new();
                if stderr.read_to_string(&mut buf).await.is_ok() && !buf.trim().is_empty() {
                    err_msg = format!("Core process failed: {}", buf.trim());
                }
            }
            anyhow::bail!("{}", err_msg);
        }

        // Drain stdout and stderr in background tasks to prevent pipe buffer deadlock
        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    tracing::debug!(target: "singbox", "{}", line);
                }
            });
        }
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    tracing::warn!(target: "singbox", "{}", line);
                }
            });
        }

        *self.child.lock().await = Some(child);
        *self.config_file.lock().await = Some(tmp);

        Ok(())
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        let mut child_guard = self.child.lock().await;
        if let Some(mut child) = child_guard.take() {
            info!("Stopping proxy core process gracefully...");
            #[cfg(unix)]
            if let Some(pid) = child.id() {
                let _ = std::process::Command::new("kill")
                    .arg("-15")
                    .arg(pid.to_string())
                    .status();
            }

            // Wait up to 1500ms for graceful exit so sing-box removes TUN routes & closes fd
            let mut exited = false;
            for _ in 0..15 {
                if let Ok(Some(_)) = child.try_wait() {
                    exited = true;
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            if !exited {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
        }

        let mut cfg_guard = self.config_file.lock().await;
        let _ = cfg_guard.take();

        self.clean_stale_process();
        cleanup_tun_stale();

        Ok(())
    }

    fn clean_stale_process(&self) {
        if let Ok(content) = std::fs::read_to_string(&self.pid_file) {
            if let Ok(pid) = content.trim().parse::<i32>() {
                let _ = std::process::Command::new("kill")
                    .arg("-15")
                    .arg(pid.to_string())
                    .status();
            }
            let _ = std::fs::remove_file(&self.pid_file);
        }
        let _ = std::process::Command::new("pkill")
            .args(["-15", "-f", "sing-box"])
            .status();
    }

    pub async fn is_running(&self) -> bool {
        let mut child_guard = self.child.lock().await;
        if let Some(ref mut child) = *child_guard
            && matches!(child.try_wait(), Ok(None))
        {
            return true;
        }

        if let Ok(content) = std::fs::read_to_string(&self.pid_file)
            && let Ok(pid) = content.trim().parse::<i32>()
            && let Ok(status) = std::process::Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .status()
        {
            return status.success();
        }

        false
    }
}
