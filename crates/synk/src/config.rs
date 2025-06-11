use ::std::path::PathBuf;
use ::std::process::{Command, Stdio};
use ::std::sync::Arc;
use ::std::sync::atomic::{AtomicBool, Ordering};
use ::std::time::{Duration, SystemTime};

use ::anyhow::{Context, Result};
use ::tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct ScriptConfig {
    pub path: PathBuf,
    pub interpreter: Option<String>,
    pub interval_seconds: u64,
    pub last_run: Option<SystemTime>,
    pub enabled: bool,
    running: Arc<AtomicBool>,
}

impl ScriptConfig {
    pub fn new(
        path: PathBuf,
        interpreter: Option<String>,
        interval_seconds: u64,
    ) -> Self {
        Self {
            path,
            interpreter,
            interval_seconds,
            last_run: None,
            enabled: true,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn should_run(&self) -> bool {
        if !self.enabled || self.is_running() {
            return false;
        }

        match self.last_run {
            None => true,
            Some(last) => {
                let elapsed = SystemTime::now()
                    .duration_since(last)
                    .unwrap_or(Duration::from_secs(0));
                elapsed >= Duration::from_secs(self.interval_seconds)
            },
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub async fn execute(&mut self) -> Result<()> {
        if self.is_running() {
            debug!("Script '{}' is already running, skipping", self.path.display());
            return Ok(());
        }

        info!("Executing script: {}", self.path.display());
        self.running.store(true, Ordering::Relaxed);

        let result = self.execute_internal().await;

        self.running.store(false, Ordering::Relaxed);
        self.last_run = Some(SystemTime::now());

        result
    }

    async fn execute_internal(&mut self) -> Result<()> {
        let mut cmd = if let Some(ref interpreter) = self.interpreter {
            let mut c = Command::new(interpreter);
            c.arg(&self.path);
            c
        } else {
            Command::new(&self.path)
        };

        // Configure command for proper signal handling
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let child = cmd.spawn().with_context(|| {
            format!("Failed to spawn script: {}", self.path.display())
        })?;

        // Wait for the process to complete
        let output = child.wait_with_output().with_context(|| {
            format!("Failed to wait for script completion: {}", self.path.display())
        })?;

        if output.status.success() {
            info!("Script executed successfully: {}", self.path.display());

            if !output.stdout.is_empty() {
                debug!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            }
        } else {
            error!("Script failed: {}", self.path.display());

            if !output.stderr.is_empty() {
                error!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            }
        }

        Ok(())
    }

    pub async fn force_terminate(&mut self) {
        if !self.is_running() {
            return;
        }

        warn!("Force terminating script: {}", self.path.display());
        // In a real implementation, you'd track the Child process
        // and call child.kill() here
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
