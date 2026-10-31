use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByeDpiPreset {
    /// Default TLS record split at SNI (-r 1+s)
    Standard,
    /// Preset tuned for Turkish ISPs / Vodafone DPI circumvention
    /// (-s1 -At -d2 -f-1 -r1+s -An)
    Vodafone,
    /// Custom args
    Custom(Vec<String>),
}

impl ByeDpiPreset {
    pub fn to_args(&self) -> Vec<String> {
        match self {
            Self::Standard => vec!["-r".into(), "1+s".into()],
            Self::Vodafone => vec![
                "-s1".into(),
                "-At".into(),
                "-d2".into(),
                "-f-1".into(),
                "-r1+s".into(),
                "-An".into(),
            ],
            Self::Custom(args) => args.clone(),
        }
    }

    pub fn from_str_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "vodafone" | "vf" => Self::Vodafone,
            _ => Self::Standard,
        }
    }
}

pub fn resolve_ciadpi_bin(custom: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = custom
        && p.exists()
    {
        return Some(p.to_path_buf());
    }

    // Check environment variable
    if let Ok(env_path) = std::env::var("BYEDPI_BIN") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return Some(p);
        }
    }

    // Check user home known locations
    if let Some(home) = dirs::home_dir() {
        let p1 = home.join("byedpi_throne_linux").join("ciadpi");
        if p1.exists() {
            return Some(p1);
        }
        let p2 = home.join(".local").join("bin").join("ciadpi");
        if p2.exists() {
            return Some(p2);
        }
    }

    // Check beside current executable
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let p = parent.join("ciadpi");
        if p.exists() {
            return Some(p);
        }
    }

    // Check current working directory
    let cwd_ciadpi = PathBuf::from("./ciadpi");
    if cwd_ciadpi.exists() {
        return Some(cwd_ciadpi);
    }

    // Check system paths
    for sys in &["/usr/local/bin/ciadpi", "/usr/bin/ciadpi", "/bin/ciadpi"] {
        let p = PathBuf::from(sys);
        if p.exists() {
            return Some(p);
        }
    }

    // Check PATH environment
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("ciadpi");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

pub struct ByeDpiRunner {
    configured_path: String,
    child: Arc<Mutex<Option<Child>>>,
    pid_file: PathBuf,
}

impl ByeDpiRunner {
    pub fn new(configured_path: String) -> Self {
        let pid_file = std::env::temp_dir().join("throne-ciadpi.pid");
        Self {
            configured_path,
            child: Arc::new(Mutex::new(None)),
            pid_file,
        }
    }

    pub async fn start(
        &self,
        custom_bin: Option<&Path>,
        custom_args: &[String],
    ) -> anyhow::Result<()> {
        self.stop().await?;

        let bin_candidate = if !self.configured_path.is_empty() {
            Some(PathBuf::from(&self.configured_path))
        } else {
            None
        };

        let bin_path = resolve_ciadpi_bin(custom_bin.or(bin_candidate.as_deref())).ok_or_else(
            || {
                anyhow::anyhow!(
                    "ByeDPI binary ('ciadpi') could not be found. Please ensure it is in PATH, ~/byedpi_throne_linux/ciadpi, or set in settings."
                )
            },
        )?;

        // Ensure clean port 1080 before spawning
        self.clean_stale_process();

        // Construct command line arguments
        // Default to binding to 127.0.0.1:1080 if not specified
        let mut final_args = Vec::new();
        let has_ip = custom_args.iter().any(|a| a == "-i" || a == "--ip");
        let has_port = custom_args.iter().any(|a| a == "-p" || a == "--port");

        if !has_ip {
            final_args.extend(["-i".to_string(), "127.0.0.1".to_string()]);
        }
        if !has_port {
            final_args.extend(["-p".to_string(), "1080".to_string()]);
        }
        final_args.extend_from_slice(custom_args);

        info!(
            "Starting ByeDPI supervisor: {:?} with args {:?}",
            bin_path, final_args
        );

        let child = match Command::new(&bin_path)
            .args(&final_args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                anyhow::bail!("Failed to spawn ciadpi from '{:?}': {}", bin_path, e);
            }
        };

        if let Some(pid) = child.id() {
            let _ = std::fs::write(&self.pid_file, pid.to_string());
        }

        *self.child.lock().await = Some(child);

        // Wait up to 1.5 seconds for port 1080 to become available
        let start = std::time::Instant::now();
        let mut ready = false;
        while start.elapsed() < Duration::from_millis(1500) {
            if TcpStream::connect("127.0.0.1:1080").await.is_ok() {
                ready = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        if ready {
            info!("ByeDPI (ciadpi) is ready and listening on 127.0.0.1:1080");
        } else {
            warn!("ByeDPI (ciadpi) spawned, but port 1080 not responding yet");
        }

        Ok(())
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        let mut child_guard = self.child.lock().await;
        if let Some(mut child) = child_guard.take() {
            info!("Stopping ByeDPI supervisor process...");
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        self.clean_stale_process();
        Ok(())
    }

    fn clean_stale_process(&self) {
        if let Ok(content) = std::fs::read_to_string(&self.pid_file) {
            if let Ok(pid) = content.trim().parse::<i32>() {
                // Send SIGTERM to previous pid
                let _ = std::process::Command::new("kill")
                    .arg("-15")
                    .arg(pid.to_string())
                    .status();
            }
            let _ = std::fs::remove_file(&self.pid_file);
        }

        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("pkill")
                .arg("-f")
                .arg("ciadpi")
                .status();
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
