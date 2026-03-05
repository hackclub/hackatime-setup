use std::path::PathBuf;
use std::process::Command;

use color_eyre::{Result, eyre::eyre};
use which::which;

use super::EditorPlugin;

pub struct TerminalWakaTime;

impl TerminalWakaTime {
    const INSTALL_URLS: [&'static str; 2] = [
        "https://hack.club/tw.sh",
        "https://hack.club/terminal-wakatime.sh",
    ];
    ];

    fn has_supported_shell() -> bool {
        ["bash", "zsh", "fish"]
            .iter()
            .any(|shell| which(shell).is_ok())
    }

    fn has_bash() -> bool {
        which("bash").is_ok()
    }

    fn install_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".wakatime/terminal-wakatime"))
    }

    fn run_installer_script(url: &str) -> Result<(bool, String)> {
        let output = Command::new("bash")
            .arg("-lc")
            .arg(format!("set -o pipefail; curl -fsSL {url} | bash"))
            .output()
            .map_err(|e| eyre!("Failed to run terminal-wakatime installer: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}\n{stderr}");

        if output.status.success()
            || combined
                .to_lowercase()
                .contains("already installed")
        {
            Ok((true, combined))
        } else {
            Ok((false, combined))
        }
    }
}

impl EditorPlugin for TerminalWakaTime {
    fn name(&self) -> String {
        "Terminal (bash/zsh/fish)".to_string()
    }

    fn is_installed(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            false
        }

        #[cfg(not(target_os = "windows"))]
        {
            Self::has_supported_shell()
        }
    }

    fn install(&self) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            return Err(eyre!(
                "terminal-wakatime setup is not currently supported on Windows (requires bash, zsh, or fish)"
            ));
        }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if !Self::has_supported_shell() {
                return Err(eyre!("No supported shell found (bash, zsh, fish)"));
            }

            if !Self::has_bash() {
                return Err(eyre!(
                    "bash is required to run the terminal-wakatime installer"
                ));
            }

            if Self::install_path().is_some_and(|p| p.exists()) {
                return Ok(());
            }

            let mut last_error_output = String::new();

            for url in Self::INSTALL_URLS {
                let (success, output) = Self::run_installer_script(url)?;
                if success {
                    return Ok(());
                }
                last_error_output = output;
            }

            Err(eyre!(
                "terminal-wakatime installer failed. Details: {}",
                last_error_output.trim()
            ))
        }
    }
}