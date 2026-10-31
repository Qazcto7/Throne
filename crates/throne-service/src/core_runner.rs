use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::info;

pub struct CoreRunner {
    core_path: String,
    child: Arc<Mutex<Option<Child>>>,
    config_file: Arc<Mutex<Option<tempfile::NamedTempFile>>>,
    pid_file: PathBuf,
}

impl CoreRunner {
    pub fn new(core_path: String) -> Self {
        let pid_file = std::env::temp_dir().join("throne-singbox.pid");
        Self {
            core_path,
            child: Arc::new(Mutex::new(None)),
            config_file: Arc::new(Mutex::new(None)),
            pid_file,
        }
    }

    pub async fn start(&self, config_json: &serde_json::Value) -> anyhow::Result<()> {
        self.stop().await?;

        // Write config to temporary file
        let mut tmp = tempfile::Builder::new()
            .prefix("throne-singbox-")
            .suffix(".json")
            .tempfile()?;

        use std::io::Write;
        tmp.write_all(serde_json::to_string_pretty(config_json)?.as_bytes())?;
        let config_path = tmp.path().to_path_buf();

        info!("Starting core proxy with config at {:?}", config_path);

        let child = match Command::new(&self.core_path)
            .arg("run")
            .arg("-c")
            .arg(&config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                anyhow::bail!("Failed to spawn core process '{}': {}", self.core_path, e);
            }
        };

        if let Some(pid) = child.id() {
            let _ = std::fs::write(&self.pid_file, pid.to_string());
        }

        *self.child.lock().await = Some(child);
        *self.config_file.lock().await = Some(tmp);

        Ok(())
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        let mut child_guard = self.child.lock().await;
        if let Some(mut child) = child_guard.take() {
            info!("Stopping proxy core process...");
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        let mut cfg_guard = self.config_file.lock().await;
        let _ = cfg_guard.take();

        self.clean_stale_process();

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
