use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::info;

pub struct CoreRunner {
    core_path: String,
    child: Arc<Mutex<Option<Child>>>,
    config_file: Arc<Mutex<Option<tempfile::NamedTempFile>>>,
}

impl CoreRunner {
    pub fn new(core_path: String) -> Self {
        Self {
            core_path,
            child: Arc::new(Mutex::new(None)),
            config_file: Arc::new(Mutex::new(None)),
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

        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        let mut child_guard = self.child.lock().await;
        if let Some(ref mut child) = *child_guard {
            matches!(child.try_wait(), Ok(None))
        } else {
            false
        }
    }
}
