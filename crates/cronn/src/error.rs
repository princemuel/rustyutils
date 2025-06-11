use ::thiserror::Error;

#[derive(Error, Debug)]
pub enum CronRunnerError {
    #[error("Script not found: {0}")]
    ScriptNotFound(String),

    #[error("Path is not a file: {0}")]
    NotAFile(String),

    #[error("No execute permission for script: {0}")]
    NoExecutePermission(String),

    #[error("Script execution failed")]
    ExecutionFailed,

    #[error("Invalid interval specified")]
    InvalidInterval,

    #[error("Invalid environment variable format: {0}")]
    InvalidEnvVarFormat(String),

    #[error("Process already running with PID {0}")]
    AlreadyRunning(u32),

    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("PID file error: {0}")]
    PidFileError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Signal handling error: {0}")]
    SignalError(String),
}
