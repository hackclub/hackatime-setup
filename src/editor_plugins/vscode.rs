use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::{Result, eyre::eyre};

use super::EditorPlugin;

pub struct VsCodeFamily {
    pub name: &'static str,
    pub config_subdir: &'static str,
    pub cli_command: &'static str,
    #[allow(dead_code)] // The dead_code lint triggers on non-Mac platforms
    pub macos_app_name: &'static str,
    #[allow(dead_code)] // The dead_code lint triggers on non-Windows platforms
    pub windows_app_folder: &'static str,
}

impl VsCodeFamily {
    fn extensions_dir(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        Some(home.join(self.config_subdir).join("extensions"))
    }

    fn get_cli_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        #[cfg(target_os = "macos")]
        {
            paths.push(PathBuf::from(format!(
                "/Applications/{}.app/Contents/Resources/app/bin/{}",
                self.macos_app_name, self.cli_command
            )));
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join(format!(
                    "Applications/{}.app/Contents/Resources/app/bin/{}",
                    self.macos_app_name, self.cli_command
                )));
            }
        }

        #[cfg(target_os = "linux")]
        {
            paths.push(PathBuf::from(format!("/usr/bin/{}", self.cli_command)));
            paths.push(PathBuf::from(format!(
                "/usr/local/bin/{}",
                self.cli_command
            )));
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join(format!(".local/bin/{}", self.cli_command)));
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
                paths.push(PathBuf::from(format!(
                    "{}/Programs/{}/bin/{}.cmd",
                    localappdata, self.windows_app_folder, self.cli_command
                )));
            }
            if let Ok(programfiles) = std::env::var("ProgramFiles") {
                paths.push(PathBuf::from(format!(
                    "{}/{}/bin/{}.cmd",
                    programfiles, self.windows_app_folder, self.cli_command
                )));
            }
        }

        paths
    }

    fn find_cli(&self) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        let cmd_ext = ".cmd";
        #[cfg(not(target_os = "windows"))]
        let cmd_ext = "";

        if let Ok(output) = Command::new("which")
            .arg(format!("{}{}", self.cli_command, cmd_ext))
            .output()
            && output.status.success()
        {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }

        #[cfg(target_os = "windows")]
        if let Ok(output) = Command::new("where").arg(self.cli_command).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .map(|s| s.trim().to_string());
                if let Some(p) = path {
                    if !p.is_empty() {
                        return Some(PathBuf::from(p));
                    }
                }
            }
        }

        self.get_cli_paths().into_iter().find(|path| path.exists())
    }
}

impl EditorPlugin for VsCodeFamily {
    fn name(&self) -> String {
        self.name.to_string()
    }

    fn is_installed(&self) -> bool {
        self.extensions_dir()
            .and_then(|d| d.parent().map(Path::exists))
            .unwrap_or(false)
            || self.find_cli().is_some()
    }

    fn install(&self) -> Result<()> {
        let cli = self
            .find_cli()
            .ok_or_else(|| eyre!("{} CLI not found", self.name))?;

        let status = Command::new(&cli)
            .args(["--install-extension", "WakaTime.vscode-wakatime"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(eyre!(
                "Failed to install WakaTime extension for {}",
                self.name
            ))
        }
    }
}
