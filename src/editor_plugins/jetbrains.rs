use std::path::PathBuf;
use std::process::Command;

use color_eyre::{Result, eyre::eyre};
use colored::Colorize;
use which::which;

use super::EditorPlugin;
use super::utils::is_process_running;

pub struct JetBrainsFamily {
    pub name: &'static str,
    pub product_codes: &'static [&'static str],
    pub cli_command: &'static str,
    #[allow(dead_code)]
    pub macos_app_names: &'static [&'static str],
}

impl JetBrainsFamily {
    fn config_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        let base_path = {
            #[cfg(target_os = "macos")]
            {
                dirs::home_dir().map(|h| h.join("Library/Application Support/JetBrains"))
            }
            #[cfg(target_os = "linux")]
            {
                dirs::home_dir().map(|h| h.join(".config/JetBrains"))
            }
            #[cfg(target_os = "windows")]
            {
                std::env::var("APPDATA")
                    .ok()
                    .map(|p| PathBuf::from(p).join("JetBrains"))
            }
        };

        if let Some(base) = base_path
            && let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    if self.product_codes.iter().any(|code| name.starts_with(code)) {
                        dirs.push(path);
                    }
                }
            }
        dirs
    }

    fn get_fallback_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        #[cfg(target_os = "macos")]
        {
            for app_name in self.macos_app_names {
                let suffix = format!(
                    "Applications/{}.app/Contents/MacOS/{}",
                    app_name, self.cli_command
                );
                paths.push(PathBuf::from(format!("/{}", suffix)));
                if let Some(home) = dirs::home_dir() {
                    paths.push(home.join(suffix));
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            paths.push(PathBuf::from(format!(
                "/opt/{}/bin/{}",
                self.cli_command, self.cli_command
            )));
            paths.push(PathBuf::from(format!(
                "/usr/local/bin/{}",
                self.cli_command
            )));
            paths.push(PathBuf::from(format!("/snap/bin/{}", self.cli_command)));

            if let Some(home) = dirs::home_dir() {
                paths.push(home.join(format!(
                    ".local/share/JetBrains/Toolbox/apps/{}/bin/{}",
                    self.cli_command, self.cli_command
                )));
            }
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
        if let Ok(path) = which(self.cli_command) {
            return Some(path);
        }
        self.get_fallback_paths()
            .into_iter()
            .find(|path| path.exists())
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

        let cli_path = self
            .find_cli()
            .ok_or_else(|| eyre!("{} CLI not found", self.name))?;

        let mut cmd;

        #[cfg(target_os = "windows")]
        {
            cmd = Command::new("cmd");
            cmd.arg("/C");
            cmd.arg(&cli_path);
        }

        #[cfg(not(target_os = "windows"))]
        {
            cmd = Command::new(&cli_path);
        }

        let status = cmd
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
