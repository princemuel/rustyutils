use ::std::collections::HashMap;
use ::std::sync::Arc;
use ::std::sync::atomic::{AtomicBool, Ordering};
use ::std::time::Duration;

use ::tokio::signal;
use ::tokio::sync::broadcast;
use ::tokio::time::sleep;
use ::tracing::{debug, error, info, warn};

use crate::config::ScriptConfig;

#[derive(Debug, Default, Clone)]
pub struct ScriptSyncer {
    scripts: HashMap<String, ScriptConfig>,
    shutdown_tx: Option<broadcast::Sender<()>>,
    is_running: Arc<AtomicBool>,
}

impl ScriptSyncer {
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
            shutdown_tx: None,
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn add_script(&mut self, name: String, config: ScriptConfig) {
        info!(
            "Adding script '{}' with interval {}s",
            name, config.interval_seconds
        );
        self.scripts.insert(name, config);
    }

    pub fn remove_script(&mut self, name: &str) -> bool {
        self.scripts.remove(name).is_some()
    }

    pub fn get_script(&self, name: &str) -> Option<&ScriptConfig> {
        self.scripts.get(name)
    }

    pub fn get_script_mut(&mut self, name: &str) -> Option<&mut ScriptConfig> {
        self.scripts.get_mut(name)
    }

    pub fn list_scripts(&self) -> Vec<(&String, &ScriptConfig)> {
        self.scripts.iter().collect()
    }

    pub fn enable_script(&mut self, name: &str, enabled: bool) -> bool {
        if let Some(script) = self.scripts.get_mut(name) {
            if enabled {
                script.enable();
            } else {
                script.disable();
            }
            true
        } else {
            false
        }
    }

    pub fn script_count(&self) -> usize {
        self.scripts.len()
    }

    pub fn enabled_script_count(&self) -> usize {
        self.scripts.values().filter(|s| s.is_enabled()).count()
    }

    pub async fn run_cycle(&mut self) {
        for (name, script) in self.scripts.iter_mut() {
            if script.should_run() {
                debug!("Running script: {}", name);
                if let Err(e) = script.execute().await {
                    error!("Error executing script '{}': {}", name, e);
                }
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    pub fn shutdown(&mut self) {
        info!("Initiating graceful shutdown...");
        self.is_running.store(false, Ordering::Relaxed);

        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
        }
    }

    pub async fn start(&mut self) {
        info!("Starting script syncer with {} scripts", self.scripts.len());

        // Set up shutdown signaling
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx);
        self.is_running.store(true, Ordering::Relaxed);

        // Set up signal handlers for graceful shutdown
        let is_running_clone = Arc::clone(&self.is_running);
        let shutdown_tx_clone = self.shutdown_tx.as_ref().unwrap().clone();

        tokio::spawn(async move {
            let mut sigterm =
                match signal::unix::signal(signal::unix::SignalKind::terminate()) {
                    Ok(signal) => signal,
                    Err(e) => {
                        warn!("Failed to register SIGTERM handler: {}", e);
                        return;
                    },
                };

            let mut sigint =
                match signal::unix::signal(signal::unix::SignalKind::interrupt()) {
                    Ok(signal) => signal,
                    Err(e) => {
                        warn!("Failed to register SIGINT handler: {}", e);
                        return;
                    },
                };

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating graceful shutdown...");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT (Ctrl+C), initiating graceful shutdown...");
                }
            }

            is_running_clone.store(false, Ordering::Relaxed);
            let _ = shutdown_tx_clone.send(());
        });

        // Main execution loop
        loop {
            tokio::select! {
                _ = self.run_cycle() => {
                    // Cycle completed normally
                }
                _ = sleep(Duration::from_secs(1)) => {
                    // Sleep completed, continue loop
                }
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received, stopping execution loop");
                    break;
                }
            }

            // Check if we should continue running
            if !self.is_running.load(Ordering::Relaxed) {
                break;
            }
        }

        // Wait for any running scripts to complete
        self.wait_for_running_scripts().await;

        info!("Script syncer shutdown complete");
    }

    async fn wait_for_running_scripts(&mut self) {
        info!("Waiting for running scripts to complete...");

        // Give scripts up to 30 seconds to complete gracefully
        let timeout = Duration::from_secs(30);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < timeout {
            let mut any_running = false;

            for (name, script) in self.scripts.iter() {
                if script.is_running() {
                    any_running = true;
                    debug!("Waiting for script '{}' to complete", name);
                }
            }

            if !any_running {
                info!("All scripts have completed");
                return;
            }

            sleep(Duration::from_millis(500)).await;
        }

        warn!("Timeout waiting for scripts to complete, forcing shutdown");

        // Force terminate any remaining scripts
        for (name, script) in self.scripts.iter_mut() {
            if script.is_running() {
                warn!("Force terminating script '{}'", name);
                script.force_terminate().await;
            }
        }
    }
}
