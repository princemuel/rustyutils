use ::std::collections::HashMap;
use ::std::path::PathBuf;
use ::std::process;
use ::std::sync::Arc;
use ::std::sync::atomic::{AtomicBool, Ordering};

use ::anyhow::Result;
use ::tracing::{Level, error, info, warn};
use ::tracing_subscriber;

use synk::{
    Args, Commands, InteractiveMode, ListFormat, ScriptConfig, ScriptSyncer,
    detect_interpreter,
};

#[tokio::main]
async fn main() {
    let args = Args::parse_args();

    // Initialize logging
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt().with_max_level(log_level).with_target(false).init();

    // Validate arguments
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
    // Set up graceful shutdown flag
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = Arc::clone(&shutdown_flag);

    // Set up Ctrl+C handler
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Received Ctrl+C, initiating shutdown...");
                shutdown_flag_clone.store(true, Ordering::Relaxed);
            },
            Err(err) => {
                error!("Failed to listen for Ctrl+C: {}", err);
            },
        }
    });

    // Execute the command
    if let Err(e) = run(args, shutdown_flag).await {
        error!("Command failed: {}", e);
        process::exit(1);
    }
}

async fn run(args: Args, shutdown_flag: Arc<AtomicBool>) -> Result<()> {
    let mut syncer = ScriptSyncer::new();

    // Load configuration if specified
    if let Some(config_path) = &args.config {
        if config_path.exists() {
            info!("Loading configuration from: {}", config_path.display());
            syncer.load_config(config_path)?;
        }
    }

    match args.command {
        Commands::Run {
            script,
            interval,
            interpreter,
            name,
            once,
            workdir,
            env,
            timeout,
        } => {
            let script_name = name.unwrap_or_else(|| {
                script
                    .file_name()
                    .unwrap_or_else(|| script.as_os_str())
                    .to_string_lossy()
                    .to_string()
            });

            let interpreter = interpreter.or_else(|| detect_interpreter(&script));
            let env_vars = Args::parse_env_vars(&env)?;

            let mut config =
                ScriptConfig::new(script.clone(), interpreter.clone(), interval);
            config.set_working_directory(workdir);
            config.set_environment_vars(env_vars);
            if let Some(t) = timeout {
                config.set_timeout(std::time::Duration::from_secs(t));
            }

            syncer.add_script(script_name.clone(), config);

            info!("Running script '{}'", script_name);
            info!("  Path: {}", script.display());
            info!("  Interval: {}s", interval);
            info!(
                "  Interpreter: {}",
                interpreter.unwrap_or_else(|| "direct".to_string())
            );

            if once {
                syncer.run_cycle().await;
                info!("Script execution completed");
            } else {
                info!("Starting continuous execution (Press Ctrl+C to stop)");
                tokio::select! {
                    _ = syncer.start() => {}
                    _ = wait_for_shutdown(shutdown_flag) => {
                        syncer.shutdown();
                    }
                }
            }
        },

        Commands::Add {
            script,
            name,
            interval,
            interpreter,
            disabled,
            workdir,
            env,
            timeout,
            priority,
            depends_on,
        } => {
            let script_name = name.unwrap_or_else(|| {
                script
                    .file_name()
                    .unwrap_or_else(|| script.as_os_str())
                    .to_string_lossy()
                    .to_string()
            });

            let interpreter = interpreter.or_else(|| detect_interpreter(&script));
            let env_vars = Args::parse_env_vars(&env)?;

            let mut config =
                ScriptConfig::new(script.clone(), interpreter.clone(), interval);
            config.set_working_directory(workdir);
            config.set_environment_vars(env_vars);
            config.set_priority(priority);
            config.set_dependencies(depends_on);

            if let Some(t) = timeout {
                config.set_timeout(std::time::Duration::from_secs(t));
            }

            if disabled {
                config.disable();
            }

            syncer.add_script(script_name.clone(), config);

            // Save configuration
            if let Some(config_path) = args.config {
                syncer.save_config(&config_path)?;
                info!("Configuration saved to: {}", config_path.display());
            }

            info!("Script '{}' added successfully", script_name);
        },

        Commands::Remove { name, force } => {
            if !force {
                print!(
                    "Are you sure you want to remove script '{}'? (y/N): ",
                    name
                );
                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().to_lowercase().starts_with('y') {
                    info!("Operation cancelled");
                    return Ok(());
                }
            }

            if syncer.remove_script(&name) {
                info!("Script '{}' removed successfully", name);

                if let Some(config_path) = args.config {
                    syncer.save_config(&config_path)?;
                }
            } else {
                error!("Script '{}' not found", name);
                process::exit(1);
            }
        },

        Commands::List { enabled, disabled, format, verbose } => {
            let scripts = syncer.list_scripts();

            let filtered_scripts: Vec<_> = scripts
                .into_iter()
                .filter(|(_, config)| {
                    if enabled && !config.is_enabled() {
                        return false;
                    }
                    if disabled && config.is_enabled() {
                        return false;
                    }
                    true
                })
                .collect();

            if filtered_scripts.is_empty() {
                info!("No scripts found matching criteria");
                return Ok(());
            }

            match format {
                ListFormat::Table => print_table(&filtered_scripts, verbose),
                ListFormat::Json => print_json(&filtered_scripts)?,
                ListFormat::Yaml => print_yaml(&filtered_scripts)?,
                ListFormat::Csv => print_csv(&filtered_scripts),
            }
        },

        Commands::Enable { names } => {
            handle_enable_disable(&mut syncer, &names, true, &args.config).await?;
        },

        Commands::Disable { names } => {
            handle_enable_disable(&mut syncer, &names, false, &args.config).await?;
        },

        Commands::Status { detailed, refresh } => {
            if refresh > 0 {
                loop {
                    print_status(&syncer, detailed);
                    tokio::time::sleep(tokio::time::Duration::from_secs(refresh))
                        .await;

                    if shutdown_flag.load(Ordering::Relaxed) {
                        break;
                    }
                }
            } else {
                print_status(&syncer, detailed);
            }
        },

        Commands::Start { foreground, pid_file, log_file } => {
            if foreground {
                info!("Starting syncer in foreground mode");
                tokio::select! {
                    _ = syncer.start() => {}
                    _ = wait_for_shutdown(shutdown_flag) => {
                        syncer.shutdown();
                    }
                }
            } else {
                info!("Daemon mode not yet implemented");
                // TODO: Implement daemon mode with pid_file and log_file
            }
        },

        Commands::Interactive => {
            info!("Starting interactive mode");
            tokio::select! {
                result = InteractiveMode::run(&mut syncer) => result?,
                _ = wait_for_shutdown(shutdown_flag) => {
                    info!("Shutdown requested, exiting interactive mode");
                    syncer.shutdown();
                }
            }
        },

        Commands::Test { script, dry_run } => {
            info!("Testing script configuration: {}", script);
            // TODO: Implement script testing
        },

        Commands::Export { output, format, include_disabled } => {
            info!("Export functionality not yet implemented");
            // TODO: Implement configuration export
        },

        Commands::Import { input, format, merge, force } => {
            info!("Import functionality not yet implemented");
            // TODO: Implement configuration import
        },

        _ => {
            info!("Command not yet fully implemented");
        },
    }

    Ok(())
}

async fn handle_enable_disable(
    syncer: &mut ScriptSyncer,
    names: &[String],
    enable: bool,
    config_path: &Option<PathBuf>,
) -> Result<()> {
    let action = if enable { "enable" } else { "disable" };
    let mut changed = false;

    for name in names {
        if name == "all" {
            let all_scripts: Vec<String> = syncer
                .list_scripts()
                .into_iter()
                .map(|(name, _)| name.clone())
                .collect();

            for script_name in all_scripts {
                if syncer.enable_script(&script_name, enable) {
                    info!(
                        "Script '{}' {}",
                        script_name,
                        if enable { "enabled" } else { "disabled" }
                    );
                    changed = true;
                }
            }
        } else if syncer.enable_script(name, enable) {
            info!(
                "Script '{}' {}",
                name,
                if enable { "enabled" } else { "disabled" }
            );
            changed = true;
        } else {
            warn!("Script '{}' not found", name);
        }
    }

    if changed {
        if let Some(config_path) = config_path {
            syncer.save_config(config_path)?;
        }
    }

    Ok(())
}

fn print_table(scripts: &[(&String, &ScriptConfig)], verbose: bool) {
    println!(
        "┌─────────────────────────┬──────────┬──────────┬─────────────────────────────────────┐"
    );
    println!(
        "│ Name                    │ Status   │ Interval │ Path                                │"
    );
    println!(
        "├─────────────────────────┼──────────┼──────────┼─────────────────────────────────────┤"
    );

    for (name, config) in scripts {
        let status = if config.is_enabled() { "enabled" } else { "disabled" };
        let path_str = config.path.to_string_lossy();
        let truncated_path = if path_str.len() > 35 {
            format!("{}...", &path_str[..32])
        } else {
            path_str.to_string()
        };

        println!(
            "│ {:<23} │ {:<8} │ {:<8}s │ {:<35} │",
            truncate_string(name, 23),
            status,
            config.interval_seconds,
            truncated_path
        );
    }

    println!(
        "└─────────────────────────┴──────────┴──────────┴─────────────────────────────────────┘"
    );
}

fn print_json(scripts: &[(&String, &ScriptConfig)]) -> Result<()> {
    let json_data: HashMap<&String, serde_json::Value> = scripts
        .iter()
        .map(|(name, config)| {
            let value = serde_json::json!({
                "path": config.path,
                "interval_seconds": config.interval_seconds,
                "enabled": config.is_enabled(),
                "interpreter": config.interpreter
            });
            (*name, value)
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json_data)?);
    Ok(())
}

fn print_yaml(scripts: &[(&String, &ScriptConfig)]) -> Result<()> {
    // TODO: Implement YAML output (requires serde_yaml dependency)
    println!("YAML output not yet implemented");
    Ok(())
}

fn print_csv(scripts: &[(&String, &ScriptConfig)]) {
    println!("name,status,interval,path,interpreter");
    for (name, config) in scripts {
        let status = if config.is_enabled() { "enabled" } else { "disabled" };
        let interpreter = config.interpreter.as_deref().unwrap_or("auto");
        println!(
            "{},{},{},{},{}",
            name,
            status,
            config.interval_seconds,
            config.path.display(),
            interpreter
        );
    }
}

fn print_status(syncer: &ScriptSyncer, detailed: bool) {
    let total = syncer.script_count();
    let enabled = syncer.enabled_script_count();

    println!("Syncer Status:");
    println!("  Total scripts: {}", total);
    println!("  Enabled: {}", enabled);
    println!("  Disabled: {}", total - enabled);
    println!("  Running: {}", syncer.is_running());

    if detailed {
        println!("\nScript Details:");
        for (name, config) in syncer.list_scripts() {
            let status = if config.is_enabled() { "enabled" } else { "disabled" };
            let running = if config.is_running() { " (running)" } else { "" };
            println!(
                "  {} - {} {}{}",
                name,
                status,
                config.path.display(),
                running
            );
        }
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

async fn wait_for_shutdown(shutdown_flag: Arc<AtomicBool>) {
    while !shutdown_flag.load(Ordering::Relaxed) {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
