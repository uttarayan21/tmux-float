use error_stack::*;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use clap::Parser;
#[derive(Debug, clap::Parser)]
pub enum Actions {
    // Toggles the attached terminal
    Toggle {
        #[arg(short, long, default_value = "scratch")]
        session: String,
        /// Whether to change the working directory to current directory on attaching
        #[arg(short, long, default_value = "true")]
        cwd: bool,
    },
    Attach {
        #[arg(short, long, default_value = "scratch")]
        session: String,
        /// Whether to change the working directory to current directory on attaching
        #[arg(short, long, default_value = "true")]
        cwd: bool,
    },
    Detach {
        #[arg(short, long, default_value = "scratch")]
        session: String,
    },
}

pub fn main() {
    let action = Actions::parse();
    action.perform().expect("Failed to perform action");
}

impl Actions {
    pub fn perform(&self) -> Result<(), std::io::Error> {
        let tmux = Tmux::new("tmux");
        match self {
            Actions::Toggle { session, cwd } => {
                tmux.toggle_session(session, *cwd)?;
            }
            Actions::Attach { session, cwd } => {
                tmux.attach_session(session, *cwd)?;
            }
            Actions::Detach { session } => {
                tmux.detach_session(session)?;
            }
        }
        Ok(())
    }
}

pub struct Tmux {
    path: PathBuf,
}
impl Tmux {
    fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().into(),
        }
    }
    fn command(&self) -> std::process::Command {
        std::process::Command::new(&self.path)
    }

    pub fn toggle_session(&self, name: impl AsRef<str>, cwd: bool) -> Result<(), std::io::Error> {
        let name = name.as_ref();
        if self.is_attached(name)? {
            self.detach_session(name)?;
        } else {
            self.attach_session(name, cwd)?;
        }
        Ok(())
    }

    pub fn has_session(&self, name: impl AsRef<str>) -> Result<bool, std::io::Error> {
        let output = self
            .command()
            .args(["has-session", "-t", name.as_ref()])
            .output()?;
        Ok(output.status.success())
    }

    pub fn create_session(&self, name: impl AsRef<str>) -> Result<(), std::io::Error> {
        self.command()
            .args(["new-session", "-d", "-s", name.as_ref()])
            .spawn()?;
        Ok(())
    }

    pub fn attach_session(&self, name: impl AsRef<str>, cwd: bool) -> Result<(), std::io::Error> {
        let name = name.as_ref();

        let attach_command = format!(
            "tmux attach-session -t {name}{cwd}",
            cwd = cwd
                .then_some(format!(" -c {}", self.var("#{pane_current_path}")?))
                .unwrap_or_default()
        );

        if !self.has_session(name)? {
            self.create_session(name)?;
        }
        if self.is_attached(name)? {
            return Ok(());
        }
        self.command()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .args([
                "popup",
                "-d",
                "'#{pane_current_path}'",
                "-xC",
                "-yC",
                "-w95%",
                "-h95%",
                "-E",
                &attach_command,
            ])
            .spawn()?;
        Ok(())
    }

    pub fn detach_session(&self, name: impl AsRef<str>) -> Result<(), std::io::Error> {
        self.command()
            .args(["detach", "-s", name.as_ref()])
            .spawn()?;
        Ok(())
    }

    pub fn is_attached(&self, name: impl AsRef<str>) -> Result<bool, std::io::Error> {
        self.var("#{session_name}")
            .change_context_lazy(|| {
                std::io::Error::new(std::io::ErrorKind::Other, "Failed to get var")
            })
            .map(|session_name| session_name.trim() == name.as_ref())
    }

    pub fn var(&self, name: impl AsRef<str>) -> Result<String, std::io::Error> {
        let output = self
            .command()
            .args(["display-message", "-p", "-F", name.as_ref()])
            .output()?;
        assert!(output.status.success());
        String::from_utf8(output.stdout).change_context_lazy(|| {
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to get var")
        })
    }
}
