use ::std::path::PathBuf;

use ::clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "synk")]
#[command(version = "0.1.0")]
#[command(about = "Periodically runs and syncs scripts")]
#[command(
    long_about = "A powerful script scheduler that can run scripts at intervals, manage multiple scripts, and provide monitoring capabilities."
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a single script (once or continuously)
    Run {
        /// Script file to run
        script: PathBuf,

        /// Interval between runs in seconds
        #[arg(short, long, default_value = "60")]
        interval: u64,

        /// Interpreter to use (auto-detected if not specified)
        #[arg(short = 'e', long)]
        interpreter: Option<String>,

        /// Name for the script (defaults to filename)
        #[arg(short, long)]
        name: Option<String>,

        /// Run script once and exit (don't loop)
        #[arg(long)]
        once: bool,

        /// Working directory for script execution
        #[arg(short, long)]
        workdir: Option<PathBuf>,

        /// Environment variables (key=value format)
        #[arg(long, value_name = "KEY=VALUE")]
        env: Vec<String>,

        /// Maximum runtime in seconds (kill if exceeded)
        #[arg(long)]
        timeout: Option<u64>,
    },

    /// Add a script to the configuration
    Add {
        /// Script file to add
        script: PathBuf,

        /// Name for the script (defaults to filename)
        #[arg(short, long)]
        name: Option<String>,

        /// Interval between runs in seconds
        #[arg(short, long, default_value = "60")]
        interval: u64,

        /// Interpreter to use (auto-detected if not specified)
        #[arg(short = 'e', long)]
        interpreter: Option<String>,

        /// Start disabled
        #[arg(long)]
        disabled: bool,

        /// Working directory for script execution
        #[arg(short, long)]
        workdir: Option<PathBuf>,

        /// Environment variables (key=value format)
        #[arg(long, value_name = "KEY=VALUE")]
        env: Vec<String>,

        /// Maximum runtime in seconds
        #[arg(long)]
        timeout: Option<u64>,

        /// Script priority (higher runs first)
        #[arg(short, long, default_value = "0")]
        priority: i32,

        /// Scripts this depends on (must complete first)
        #[arg(long)]
        depends_on: Vec<String>,
    },

    /// Remove a script from the configuration
    Remove {
        /// Name of the script to remove
        name: String,

        /// Don't ask for confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// List all configured scripts
    List {
        /// Show only enabled scripts
        #[arg(long)]
        enabled: bool,

        /// Show only disabled scripts
        #[arg(long)]
        disabled: bool,

        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: ListFormat,

        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Enable one or more scripts
    Enable {
        /// Names of scripts to enable (or 'all' for all scripts)
        names: Vec<String>,
    },

    /// Disable one or more scripts
    Disable {
        /// Names of scripts to disable (or 'all' for all scripts)
        names: Vec<String>,
    },

    /// Show status of the syncer and scripts
    Status {
        /// Show detailed status for each script
        #[arg(short, long)]
        detailed: bool,

        /// Refresh interval in seconds (0 for single check)
        #[arg(short, long, default_value = "0")]
        refresh: u64,
    },

    /// Start the script syncer daemon
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,

        /// PID file path for daemon mode
        #[arg(long)]
        pid_file: Option<PathBuf>,

        /// Log file path for daemon mode
        #[arg(long)]
        log_file: Option<PathBuf>,
    },

    /// Stop the running syncer daemon
    Stop {
        /// PID file path
        #[arg(long)]
        pid_file: Option<PathBuf>,

        /// Force kill if graceful shutdown fails
        #[arg(short, long)]
        force: bool,
    },

    /// Restart the syncer daemon
    Restart {
        /// PID file path
        #[arg(long)]
        pid_file: Option<PathBuf>,

        /// Force kill if graceful shutdown fails
        #[arg(short, long)]
        force: bool,
    },

    /// Run interactive mode for script management
    Interactive,

    /// Show logs for a specific script or all scripts
    Logs {
        /// Script name (optional, shows all if not specified)
        script: Option<String>,

        /// Number of lines to show
        #[arg(short, long, default_value = "50")]
        lines: usize,

        /// Follow log output (like tail -f)
        #[arg(short, long)]
        follow: bool,

        /// Show logs since specific time (e.g., '1h', '30m', '2024-01-01')
        #[arg(long)]
        since: Option<String>,
    },

    /// Test a script configuration without running it
    Test {
        /// Script name or path to test
        script: String,

        /// Show what would be executed
        #[arg(long)]
        dry_run: bool,
    },

    /// Export configuration to a file
    Export {
        /// Output file path (stdout if not specified)
        output: Option<PathBuf>,

        /// Export format
        #[arg(long, value_enum, default_value = "json")]
        format: ExportFormat,

        /// Include disabled scripts
        #[arg(long)]
        include_disabled: bool,
    },

    /// Import configuration from a file
    Import {
        /// Input file path
        input: PathBuf,

        /// Import format (auto-detected if not specified)
        #[arg(long, value_enum)]
        format: Option<ExportFormat>,

        /// Merge with existing configuration
        #[arg(long)]
        merge: bool,

        /// Don't ask for confirmation
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(clap::ValueEnum, Clone)]
pub enum ListFormat {
    Table,
    Json,
    Yaml,
    Csv,
}

#[derive(clap::ValueEnum, Clone)]
pub enum ExportFormat {
    Json,
    Yaml,
    Toml,
}

impl Args {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    pub fn validate(&self) -> Result<(), String> {
        match &self.command {
            Commands::Run { script, interval, timeout, .. } => {
                if !script.exists() {
                    return Err(format!(
                        "Script file '{}' does not exist",
                        script.display()
                    ));
                }
                if *interval == 0 {
                    return Err("Interval must be greater than 0".to_string());
                }
                if let Some(t) = timeout {
                    if *t == 0 {
                        return Err("Timeout must be greater than 0".to_string());
                    }
                }
            },
            Commands::Add { script, interval, timeout, .. } => {
                if !script.exists() {
                    return Err(format!(
                        "Script file '{}' does not exist",
                        script.display()
                    ));
                }
                if *interval == 0 {
                    return Err("Interval must be greater than 0".to_string());
                }
                if let Some(t) = timeout {
                    if *t == 0 {
                        return Err("Timeout must be greater than 0".to_string());
                    }
                }
            },
            Commands::Remove { name, .. } => {
                if name.is_empty() {
                    return Err("Script name cannot be empty".to_string());
                }
            },
            Commands::Enable { names } | Commands::Disable { names } => {
                if names.is_empty() {
                    return Err("Must specify at least one script name".to_string());
                }
            },
            Commands::Import { input, .. } => {
                if !input.exists() {
                    return Err(format!(
                        "Import file '{}' does not exist",
                        input.display()
                    ));
                }
            },
            _ => {},
        }
        Ok(())
    }

    pub fn get_script_name_from_run(&self) -> Option<String> {
        match &self.command {
            Commands::Run { name, script, .. } => {
                if let Some(name) = name {
                    Some(name.clone())
                } else {
                    Some(
                        script
                            .file_name()
                            .unwrap_or_else(|| script.as_os_str())
                            .to_string_lossy()
                            .to_string(),
                    )
                }
            },
            _ => None,
        }
    }

    pub fn get_script_name_from_add(&self) -> Option<String> {
        match &self.command {
            Commands::Add { name, script, .. } => {
                if let Some(name) = name {
                    Some(name.clone())
                } else {
                    Some(
                        script
                            .file_name()
                            .unwrap_or_else(|| script.as_os_str())
                            .to_string_lossy()
                            .to_string(),
                    )
                }
            },
            _ => None,
        }
    }

    pub fn parse_env_vars(
        env_vars: &[String],
    ) -> Result<std::collections::HashMap<String, String>, String> {
        let mut env_map = std::collections::HashMap::new();

        for env_var in env_vars {
            if let Some((key, value)) = env_var.split_once('=') {
                env_map.insert(key.to_string(), value.to_string());
            } else {
                return Err(format!(
                    "Invalid environment variable format: '{}'. Use KEY=VALUE",
                    env_var
                ));
            }
        }

        Ok(env_map)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_get_script_name() {
//         let mut args = Args {
//             script: Some(PathBuf::from("test.py")),
//             interval: 60,
//             interpreter: None,
//             name: None,
//             interactive: false,
//             verbose: false,
//             once: false,
//         };

//         // Test default name from filename
//         assert_eq!(args.get_script_name(), Some("test.py".to_string()));

//         // Test custom name
//         args.name = Some("custom_name".to_string());
//         assert_eq!(args.get_script_name(), Some("custom_name".to_string()));

//         // Test no script
//         args.script = None;
//         args.name = None;
//         assert_eq!(args.get_script_name(), None);
//     }

//     #[test]
//     fn test_validate() {
//         let args = Args {
//             script: None,
//             interval: 60,
//             interpreter: None,
//             name: None,
//             interactive: true,
//             verbose: false,
//             once: false,
//         };

//         // Interactive mode should be valid
//         assert!(args.validate().is_ok());

//         // Zero interval should be invalid
//         let mut invalid_args = args.clone();
//         invalid_args.interval = 0;
//         assert!(invalid_args.validate().is_err());

//         // Interactive + script should be invalid
//         let mut invalid_args = args.clone();
//         invalid_args.script = Some(PathBuf::from("test.py"));
//         assert!(invalid_args.validate().is_err());
//     }
// }
