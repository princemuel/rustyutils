use ::std::fs;
use ::std::io::Write;
use ::std::path::{Path, PathBuf};

use ::anyhow::{Context, Result};
use ::nix::unistd::Pid;
use ::sysinfo::{Pid as SysPid, ProcessesToUpdate, System};

pub struct PidFile {
    path: PathBuf,
}

impl PidFile {
    pub fn new(script_path: &Path) -> Result<Self> {
        let pid_path = PathBuf::from(format!(
            "/tmp/cronn_{}.pid",
            script_path.file_name().unwrap().to_str().unwrap()
        ));
        Ok(Self { path: pid_path })
    }

    pub fn create(&self) -> Result<()> {
        let mut file = fs::File::create(&self.path).context(format!(
            "Failed to create PID file at {}",
            self.path.display()
        ))?;
        file.write_all(Pid::this().to_string().as_bytes())
            .context("Failed to write PID to file")?;
        Ok(())
    }

    pub fn cleanup(&self) -> Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path).context(format!(
                "Failed to remove PID file at {}",
                self.path.display()
            ))?;
        }
        Ok(())
    }

    pub fn pid(&self) -> Result<usize> {
        if !self.path.exists() {
            return Ok(0);
        }

        let pid_str = fs::read_to_string(&self.path).context(format!(
            "Failed to read PID file at {}",
            self.path.display()
        ))?;
        pid_str.trim().parse().context("Failed to parse PID from file")
    }

    pub fn is_running(&self) -> Result<bool> {
        let pid = self.pid()?;
        if pid == 0 {
            return Ok(false);
        }

        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        Ok(sys.process(SysPid::from(pid)).is_some())
    }
}
