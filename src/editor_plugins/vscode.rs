use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::{Result, eyre::eyre};
use which::which;

use super::EditorPlugin;

pub struct VsCodeFamily {
    pub name: &'static str,
    pub config_subdir: &'static str,
    pub cli_command: &'static str,
    #[allow(dead_code)]
    pub macos_app_name: &'static str,
    #[allow(dead_code)]
    pub windows_app_folder: &'static str,
}

impl VsCodeFamily {
    fn extensions_dir(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        Some(home.join(self.config_subdir).join("extensions"))
    }

    fn get_fallback_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        #[cfg(target_os = "macos")]
        {
            let app_path = format!(
                "Applications/{}.app/Contents/Resources/app/bin/{}",
                self.macos_app_name, self.cli_command
            );

            paths.push(PathBuf::from(format!("/{}", app_path)));
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join(app_path));
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Common Linux locations
            paths.push(PathBuf::from(format!("/usr/bin/{}", self.cli_command)));
            paths.push(PathBuf::from(format!(
                "/usr/local/bin/{}",
                self.cli_command
            )));
            paths.push(PathBuf::from(format!("/snap/bin/{}", self.cli_command)));
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join(format!(".local/bin/{}", self.cli_command)));
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Windows users might install to LocalAppData or Program Files
            let binary = format!("{}.cmd", self.cli_command); // Explicitly look for .cmd

            if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
                paths.push(PathBuf::from(format!(
                    "{}\\Programs\\{}\\bin\\{}",
                    localappdata, self.windows_app_folder, binary
                )));
            }

            if let Ok(program_files) = std::env::var("ProgramFiles") {
                paths.push(PathBuf::from(format!(
                    "{}\\{}\\bin\\{}",
                    program_files, self.windows_app_folder, binary
                )));
            }

            if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
                paths.push(PathBuf::from(format!(
                    "{}\\{}\\bin\\{}",
                    program_files_x86, self.windows_app_folder, binary
                )));
            }
        }

        paths
    }

    fn find_cli(&self) -> Option<PathBuf> {
        // 1. Try to find it in the System PATH using the 'which' crate.
        // This handles .cmd, .exe, and .bat automatically on Windows.
        if let Ok(path) = which(self.cli_command) {
            return Some(path);
        }

        // 2. Fallback to hardcoded paths if not in PATH
        self.get_fallback_paths()
            .into_iter()
            .find(|path| path.exists())
    }
}

impl EditorPlugin for VsCodeFamily {
    fn name(&self) -> String {
        self.name.to_string()
    }

    fn is_installed(&self) -> bool {
        // It's installed if we can find the CLI OR the extension folder exists
        self.find_cli().is_some()
            || self
                .extensions_dir()
                .and_then(|d| d.parent().map(Path::exists))
                .unwrap_or(false)
    }

    fn install(&self) -> Result<()> {
        let cli_path = self.find_cli().ok_or_else(|| {
            eyre!(
                "{} CLI not found. Is it installed and in your PATH?",
                self.name
            )
        })?;

        // Prepare the command
        let mut cmd;

        #[cfg(target_os = "windows")]
        {
            // FIX for os error 193:
            // On Windows, the 'code' command is often a .cmd batch file.
            // Executing batch files directly via Command::new sometimes fails
            // with error 193 if the OS environment isn't perfect.
            // We wrap it in `cmd /C` to guarantee execution.
            cmd = Command::new("cmd");
            cmd.arg("/C");
            cmd.arg(&cli_path);
        }

        #[cfg(not(target_os = "windows"))]
        {
            cmd = Command::new(&cli_path);
        }

        let status = cmd
            .args(["--install-extension", "WakaTime.vscode-wakatime"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| eyre!("Failed to execute {:?}: {}", cli_path, e))?;

        if status.success() {
            Ok(())
        } else {
            Err(eyre!(
                "Failed to install WakaTime extension for {}. Exit code: {:?}",
                self.name,
                status.code()
            ))
        }
    }
}
