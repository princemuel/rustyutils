use ::std::collections::HashMap;
use ::std::path::{Path, PathBuf};
use ::std::process::{Command as StdCommand, Stdio};
use ::std::time::{Duration, Instant};
use ::std::{env, fs};

use ::anyhow::{Context, Result};
use ::chrono::Local;
use ::clap::Parser;
use ::log::{debug, error, info, warn};
use ::nix::sys::stat;
use ::nix::unistd::{Gid, Uid};
use ::serde::{Deserialize, Serialize};
use ::signal_hook::consts::{SIGINT, SIGTERM};
use ::signal_hook::iterator::Signals;
use ::simplelog::{
    CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode, WriteLogger,
};
use ::sysinfo::{Pid, ProcessExt, System, SystemExt};

use ::cronn::config::Config;
use ::cronn::error::CronRunnerError;
use ::cronn::pid::PidFile;

#[derive(Parser, Debug)]
#[command(
    name = "cronn",
    version = "0.2.0",
    about = "A robust cron job runner in Rust",
    long_about = "A cron-like job runner that executes scripts at specified intervals with advanced features like timeout, PID file management, and comprehensive logging."
)]
struct Cli {
    /// Path to the script to execute
    #[arg(required = true)]
    script: PathBuf,

    /// Arguments to pass to the script
    #[arg(last = true)]
    args: Vec<String>,

    /// Interval between executions (e.g., '30s', '5m', '1h')
    #[arg(short, long, default_value = "60s", value_parser = humantime::parse_duration)]
    interval: Duration,

    /// Path to log file
    #[arg(short, long, default_value = "/var/log/cronn.log")]
    log_file: PathBuf,

    /// Configuration file path
    #[arg(short, long, default_value = "/etc/cronn/config.yaml")]
    config_file: PathBuf,

    /// Timeout for script execution (e.g., '30s', '5m')
    #[arg(short, long, value_parser = humantime::parse_duration)]
    timeout: Option<Duration>,

    /// Environment variables in KEY=VALUE format
    #[arg(short, long, value_parser = parse_key_val)]
    env: Vec<(String, String)>,

    /// Enable verbose logging (use ::-v for debug, -vv for trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Dry run - don't actually execute the script
    #[arg(long)]
    dry_run: bool,

    /// Maximum number of times to run the script (0 for unlimited)
    #[arg(long, default_value = "0")]
    max_runs: u32,
}

/// Parse KEY=VALUE environment variable pairs
fn parse_key_val(s: &str) -> Result<(String, String), CronRunnerError> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(CronRunnerError::InvalidEnvVarFormat(s.to_string()));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

#[derive(Debug, Serialize, Deserialize)]
struct JobHistory {
    runs: Vec<RunRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RunRecord {
    timestamp: String,
    duration: f64,
    exit_code: Option<i32>,
    success: bool,
    pid: u32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = match cli.verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    CombinedLogger::init(vec![
        TermLogger::new(
            log_level,
            Config::default(),
            TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        ),
        WriteLogger::new(
            log_level,
            Config::default(),
            fs::File::create(&cli.log_file).context("Failed to create log file")?,
        ),
    ])
    .context("Failed to initialize logging")?;

    info!("Starting cron runner (v{})", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Config::load(&cli.config_file).unwrap_or_default();
    debug!("Loaded configuration: {:?}", config);

    // Setup signal handling
    let mut signals = Signals::new(&[SIGINT, SIGTERM])?;
    std::thread::spawn(move || {
        for sig in signals.forever() {
            info!("Received signal {:?}, shutting down...", sig);
            std::process::exit(0);
        }
    });

    // Check for existing process
    let procid = PidFile::new(&cli.script)?;
    if procid.is_running()? {
        return Err(CronRunnerError::AlreadyRunning(
            procid.pid()?.try_into().unwrap(),
        )
        .into());
    }
    procid.create()?;

    // Validate script
    validate_script(&cli.script)?;

    // Main execution loop
    let mut run_count = 0;
    let mut job_history = JobHistory { runs: Vec::new() };

    loop {
        if cli.max_runs > 0 && run_count >= cli.max_runs {
            info!("Reached maximum run count ({}), exiting", cli.max_runs);
            break;
        }

        info!("Executing script: {}", cli.script.display());
        let start_time = Instant::now();

        if cli.dry_run {
            info!(
                "Dry run: would execute {} with args {:?}",
                cli.script.display(),
                cli.args
            );
        } else {
            match execute_script(&cli, &mut job_history, start_time) {
                Ok(output) => {
                    let duration = start_time.elapsed().as_secs_f64();
                    info!(
                        "Script executed in {:.2}s (exit code: {})",
                        duration,
                        output.status.code().unwrap_or(-1)
                    );
                    log_output(&output);
                },
                Err(e) => {
                    error!("Script execution failed: {}", e);
                },
            }
        }

        run_count += 1;
        info!("Waiting {:?} before next execution", cli.interval);
        std::thread::sleep(cli.interval);
    }

    procid.cleanup()?;
    Ok(())
}

fn execute_script(
    cli: &Cli,
    job_history: &mut JobHistory,
    start_time: Instant,
) -> Result<std::process::Output> {
    let mut command = StdCommand::new("bash");
    command.arg(&cli.script).args(&cli.args);

    // Set environment variables
    for (key, value) in &cli.env {
        command.env(key, value);
    }

    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn script process")?;

    let pid = child.id();

    // Handle timeout if specified
    let output = if let Some(timeout) = cli.timeout {
        let start = Instant::now();
        loop {
            if start.elapsed() > timeout {
                // Timeout reached, kill the process
                let mut sys = System::new();
                sys.refresh_processes();
                if let Some(process) = sys.process(Pid::from(pid as i32)) {
                    process.kill();
                }
                return Err(CronRunnerError::Timeout(timeout).into());
            }

            if let Ok(Some(status)) = child.try_wait() {
                let output = child.wait_with_output()?;
                record_execution(job_history, start_time, pid, status.code(), true);
                return Ok(output);
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    } else {
        let output = child.wait_with_output()?;
        record_execution(
            job_history,
            start_time,
            pid,
            output.status.code(),
            output.status.success(),
        );
        Ok(output)
    }?;

    Ok(output)
}

fn record_execution(
    job_history: &mut JobHistory,
    start_time: Instant,
    pid: u32,
    exit_code: Option<i32>,
    success: bool,
) {
    let record = RunRecord {
        timestamp: Local::now().to_rfc3339(),
        duration: start_time.elapsed().as_secs_f64(),
        exit_code,
        success,
        pid,
    };
    job_history.runs.push(record);
}

fn log_output(output: &std::process::Output) {
    if !output.stdout.is_empty() {
        info!("Script stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        warn!("Script stderr:\n{}", String::from_utf8_lossy(&output.stderr));
    }
}

fn validate_script(script_path: &Path) -> Result<()> {
    // Check if file exists
    if !script_path.exists() {
        return Err(CronRunnerError::ScriptNotFound(
            script_path.display().to_string(),
        )
        .into());
    }

    // Check if file is a regular file
    if !script_path.is_file() {
        return Err(
            CronRunnerError::NotAFile(script_path.display().to_string()).into()
        );
    }

    // Check execute permissions
    let metadata =
        script_path.metadata().context("Failed to get script metadata")?;
    let permissions = metadata.permissions();
    let mode = permissions.mode();

    // Check if the file is executable by the current user
    if mode & 0o111 == 0 {
        // No execute permissions for anyone
        return Err(CronRunnerError::NoExecutePermission(
            script_path.display().to_string(),
        )
        .into());
    }

    // Check if we have execute permissions
    let current_uid = Uid::current();
    let current_gid = Gid::current();

    let file_uid = Uid::from_raw(metadata.uid());
    let file_gid = Gid::from_raw(metadata.gid());

    if file_uid == current_uid {
        // Owner - check user execute bit
        if mode & 0o100 == 0 {
            return Err(CronRunnerError::NoExecutePermission(
                script_path.display().to_string(),
            )
            .into());
        }
    } else if file_gid == current_gid {
        // Group - check group execute bit
        if mode & 0o010 == 0 {
            return Err(CronRunnerError::NoExecutePermission(
                script_path.display().to_string(),
            )
            .into());
        }
    } else {
        // Others - check others execute bit
        if mode & 0o001 == 0 {
            return Err(CronRunnerError::NoExecutePermission(
                script_path.display().to_string(),
            )
            .into());
        }
    }

    Ok(())
}
