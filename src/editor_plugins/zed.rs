use std::process::Command;

#[cfg(target_os = "linux")]
use std::path::PathBuf;

use color_eyre::{Result, eyre::eyre};
use dialoguer::{Confirm, theme::ColorfulTheme};

use super::EditorPlugin;

pub struct Zed;

impl Zed {
    fn has_url_handler() -> bool {
        #[cfg(target_os = "macos")]
        {
            Command::new("/usr/bin/open")
                .args(["-Ra", "zed"])
                .output()
                .is_ok_and(|o| o.status.success())
        }

        #[cfg(target_os = "linux")]
        {
            // Try xdg-mime first, fall back to checking common install locations
            if let Ok(o) = Command::new("xdg-mime")
                .args(["query", "default", "x-scheme-handler/zed"])
                .output()
            {
                if o.status.success() && !o.stdout.is_empty() {
                    return true;
                }
            }
            // Fallback: check if zed binary exists
            [
                PathBuf::from("/usr/bin/zed"),
                PathBuf::from("/usr/bin/zeditor"),
                PathBuf::from("/usr/local/bin/zed"),
                dirs::home_dir()
                    .map(|h| h.join(".local/bin/zed"))
                    .unwrap_or_default(),
            ]
            .iter()
            .any(|p| p.exists())
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("reg")
                .args(["query", r"HKEY_CLASSES_ROOT\zed"])
                .output()
                .is_ok_and(|o| o.status.success())
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            false
        }
    }
}

impl EditorPlugin for Zed {
    fn name(&self) -> String {
        "Zed".to_string()
    }

    fn is_installed(&self) -> bool {
        Self::has_url_handler()
    }

    fn install(&self) -> Result<()> {
        let proceed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("I'll open Zed to the WakaTime extension page. Click Install, then come back here. Ready?")
            .default(true)
            .interact()?;

        if !proceed {
            return Ok(());
        }

        open::that_detached("zed://extension/wakatime")
            .map_err(|e| eyre!("Failed to open Zed extension page: {}", e))
    }
}
