use std::path::PathBuf;
use std::process::Command;

use color_eyre::{Result, eyre::eyre};
use colored::Colorize;

use super::EditorPlugin;
use super::utils::is_process_running;

pub struct JetBrainsFamily {
    pub name: &'static str,
    pub product_codes: &'static [&'static str],
    pub cli_command: &'static str,
    #[allow(dead_code)] // The dead_code lint triggers on non-Mac platforms
    pub macos_app_names: &'static [&'static str],
}

impl JetBrainsFamily {
    fn config_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        #[cfg(target_os = "macos")]
        if let Some(home) = dirs::home_dir() {
            let base = home.join("Library/Application Support/JetBrains");
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if self
                        .product_codes
                        .iter()
                        .any(|code| name_str.starts_with(code))
                    {
                        dirs.push(entry.path());
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        if let Some(home) = dirs::home_dir() {
            let base = home.join(".config/JetBrains");
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if self
                        .product_codes
                        .iter()
                        .any(|code| name_str.starts_with(code))
                    {
                        dirs.push(entry.path());
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        if let Ok(appdata) = std::env::var("APPDATA") {
            let base = PathBuf::from(appdata).join("JetBrains");
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if self
                        .product_codes
                        .iter()
                        .any(|code| name_str.starts_with(code))
                    {
                        dirs.push(entry.path());
                    }
                }
            }
        }

        dirs
    }

    fn get_cli_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        #[cfg(target_os = "macos")]
        {
            for app_name in self.macos_app_names {
                paths.push(PathBuf::from(format!(
                    "/Applications/{}.app/Contents/MacOS/{}",
                    app_name, self.cli_command
                )));
                if let Some(home) = dirs::home_dir() {
                    paths.push(home.join(format!(
                        "Applications/{}.app/Contents/MacOS/{}",
                        app_name, self.cli_command
                    )));
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join(format!(
                    ".local/share/JetBrains/Toolbox/apps/{}/bin/{}",
                    self.cli_command, self.cli_command
                )));
            }
            paths.push(PathBuf::from(format!(
                "/opt/{}/bin/{}",
                self.cli_command, self.cli_command
            )));
            paths.push(PathBuf::from(format!(
                "/usr/local/bin/{}",
                self.cli_command
            )));
            paths.push(PathBuf::from(format!("/snap/bin/{}", self.cli_command)));
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
                paths.push(PathBuf::from(format!(
                    "{}/JetBrains/Toolbox/apps/{}/bin/{}.cmd",
                    localappdata, self.cli_command, self.cli_command
                )));
            }
            if let Ok(programfiles) = std::env::var("ProgramFiles") {
                for app_name in self.macos_app_names {
                    paths.push(PathBuf::from(format!(
                        "{}/JetBrains/{}/bin/{}.bat",
                        programfiles, app_name, self.cli_command
                    )));
                }
            }
        }

        paths
    }

    fn find_cli(&self) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            // On Windows, try running the CLI to check if it exists
            if Command::new(self.cli_command)
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok()
            {
                return Some(PathBuf::from(self.cli_command));
            }
            // Fall back to known paths
            return self.get_cli_paths().into_iter().find(|path| path.exists());
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(output) = Command::new("which").arg(self.cli_command).output()
                && output.status.success()
            {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }

            self.get_cli_paths().into_iter().find(|path| path.exists())
        }
    }

    fn is_running(&self) -> bool {
        is_process_running(self.cli_command)
    }
}

impl EditorPlugin for JetBrainsFamily {
    fn name(&self) -> String {
        self.name.to_string()
    }

    fn is_installed(&self) -> bool {
        !self.config_dirs().is_empty() || self.find_cli().is_some()
    }

    fn install(&self) -> Result<()> {
        if self.is_running() {
            eprintln!(
                "{}",
                format!(
                    "Warning: {} appears to be running. Please close it for the plugin to install correctly.",
                    self.name
                ).yellow()
            );
        }

        let cli = self
            .find_cli()
            .ok_or_else(|| eyre!("{} CLI not found", self.name))?;

        let status = Command::new(&cli)
            .args(["installPlugins", "com.wakatime.intellij.plugin"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(eyre!("Failed to install WakaTime plugin for {}", self.name))
        }
    }
}
