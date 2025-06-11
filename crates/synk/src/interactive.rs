use anyhow::Result;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::config::ScriptConfig;
use crate::interpreter::detect_interpreter;
use crate::syncer::ScriptSyncer;

pub struct InteractiveMode;

impl InteractiveMode {
    pub async fn run(syncer: &mut ScriptSyncer) -> Result<()> {
        println!("Synk Interactive Mode");
        println!(
            "Commands: add, remove, list, enable, disable, start, status, help, quit"
        );

        loop {
            print!("> ");
            io::stdout().flush()?;

            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;
            let buffer = buffer.trim();

            let parts: Vec<&str> = buffer.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "add" => Self::handle_add_command(&parts, syncer),
                "remove" => Self::handle_remove_command(&parts, syncer),
                "list" => Self::handle_list_command(syncer),
                "enable" => Self::handle_enable_command(&parts, syncer, true),
                "disable" => Self::handle_enable_command(&parts, syncer, false),
                "start" => {
                    println!("Starting script syncer...");
                    syncer.start().await;
                },
                "status" => Self::handle_status_command(syncer),
                "help" => Self::show_help(),
                "quit" | "exit" => {
                    println!("Goodbye!");
                    break;
                },
                _ => {
                    println!(
                        "Unknown command. Type 'help' for available commands."
                    );
                },
            }
        }

        Ok(())
    }

    fn handle_add_command(parts: &[&str], syncer: &mut ScriptSyncer) {
        if parts.len() < 2 {
            println!("Usage: add <script_path> [interval_seconds] [interpreter]");
            return;
        }

        let path = PathBuf::from(parts[1]);

        // Check if file exists
        if !path.exists() {
            println!("Warning: File '{}' does not exist", path.display());
        }

        let interval = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(60);

        if interval == 0 {
            println!("Error: Interval must be greater than 0");
            return;
        }

        let interpreter = parts
            .get(3)
            .map(|s| s.to_string())
            .or_else(|| detect_interpreter(&path));

        let name = path
            .file_name()
            .unwrap_or_else(|| path.as_os_str())
            .to_string_lossy()
            .to_string();

        // Check if script with same name already exists
        if syncer.get_script(&name).is_some() {
            println!(
                "Warning: Script with name '{}' already exists and will be replaced",
                name
            );
        }

        let config = ScriptConfig::new(path.clone(), interpreter.clone(), interval);
        syncer.add_script(name, config);

        println!("Script added successfully:");
        println!("  Path: {}", path.display());
        println!("  Interval: {}s", interval);
        println!(
            "  Interpreter: {}",
            interpreter.unwrap_or_else(|| "auto-detect".to_string())
        );
    }

    fn handle_remove_command(parts: &[&str], syncer: &mut ScriptSyncer) {
        if parts.len() < 2 {
            println!("Usage: remove <script_name>");
            return;
        }

        if syncer.remove_script(parts[1]) {
            println!("Script '{}' removed successfully", parts[1]);
        } else {
            println!("Script '{}' not found", parts[1]);
        }
    }

    fn handle_list_command(syncer: &ScriptSyncer) {
        let scripts = syncer.list_scripts();
        if scripts.is_empty() {
            println!("No scripts configured");
        } else {
            println!("Configured scripts:");
            for (name, config) in scripts {
                let status =
                    if config.is_enabled() { "enabled" } else { "disabled" };
                let interpreter =
                    config.interpreter.as_deref().unwrap_or("auto-detect");

                println!(
                    "  {} - {} ({}s interval, {}, interpreter: {})",
                    name,
                    config.path.display(),
                    config.interval_seconds,
                    status,
                    interpreter
                );
            }
        }
    }

    fn handle_enable_command(
        parts: &[&str],
        syncer: &mut ScriptSyncer,
        enable: bool,
    ) {
        if parts.len() < 2 {
            let action = if enable { "enable" } else { "disable" };
            println!("Usage: {} <script_name>", action);
            return;
        }

        let action_word = if enable { "enabled" } else { "disabled" };

        if syncer.enable_script(parts[1], enable) {
            println!("Script '{}' {}", parts[1], action_word);
        } else {
            println!("Script '{}' not found", parts[1]);
        }
    }

    fn handle_status_command(syncer: &ScriptSyncer) {
        let total = syncer.script_count();
        let enabled = syncer.enabled_script_count();
        let disabled = total - enabled;

        println!("Script Syncer Status:");
        println!("  Total scripts: {}", total);
        println!("  Enabled: {}", enabled);
        println!("  Disabled: {}", disabled);
    }

    fn show_help() {
        println!("Available commands:");
        println!(
            "  add <script_path> [interval_seconds] [interpreter] - Add a new script"
        );
        println!(
            "  remove <script_name>                             - Remove a script"
        );
        println!(
            "  list                                             - List all scripts"
        );
        println!(
            "  enable <script_name>                             - Enable a script"
        );
        println!(
            "  disable <script_name>                            - Disable a script"
        );
        println!(
            "  start                                            - Start running all enabled scripts"
        );
        println!(
            "  status                                           - Show syncer status"
        );
        println!(
            "  help                                             - Show this help message"
        );
        println!(
            "  quit/exit                                        - Exit interactive mode"
        );
        println!();
        println!("Examples:");
        println!("  add my_script.py 30");
        println!("  add backup.sh 3600 bash");
        println!("  enable my_script.py");
    }
}
