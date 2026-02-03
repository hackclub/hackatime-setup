use std::fs;
use std::path::PathBuf;
use std::process::Command;

use color_eyre::{Result, eyre::eyre};
use jsonc_parser::{ParseOptions, cst::CstRootNode, json};

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
            if let Ok(o) = Command::new("xdg-mime")
                .args(["query", "default", "x-scheme-handler/zed"])
                .output()
            {
                if o.status.success() && !o.stdout.is_empty() {
                    return true;
                }
            }
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

    fn config_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|h| h.join(".config/zed"))
        }

        #[cfg(target_os = "linux")]
        {
            std::env::var("FLATPAK_XDG_CONFIG_HOME")
                .map(|p| PathBuf::from(p).join("zed"))
                .ok()
                .or_else(|| dirs::config_dir().map(|c| c.join("zed")))
        }

        #[cfg(target_os = "windows")]
        {
            dirs::config_dir().map(|c| c.join("Zed"))
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }

    fn add_extension_to_settings(settings_path: &PathBuf) -> Result<()> {
        let content = if settings_path.exists() {
            let s = fs::read_to_string(settings_path)
                .map_err(|e| eyre!("Failed to read {}: {}", settings_path.display(), e))?;
            if s.trim().is_empty() {
                String::from("{}")
            } else {
                s
            }
        } else {
            String::from("{}")
        };

        let root = CstRootNode::parse(&content, &ParseOptions::default())
            .map_err(|e| eyre!("Invalid {}: {}", settings_path.display(), e))?;

        let root_obj = root
            .object_value_or_create()
            .ok_or_else(|| eyre!("{} root must be an object", settings_path.display()))?;

        let extensions = root_obj
            .object_value_or_create("auto_install_extensions")
            .ok_or_else(|| eyre!("auto_install_extensions must be an object"))?;

        match extensions.get("wakatime") {
            None => {
                extensions.append("wakatime", json!(true));
            }
            Some(prop) => {
                prop.set_value(json!(true));
            }
        }

        if let Some(parent) = settings_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(settings_path, root.to_string())
            .map_err(|e| eyre!("Failed to write {}: {}", settings_path.display(), e))?;

        Ok(())
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
        let settings_path = Self::config_dir()
            .ok_or_else(|| eyre!("Could not determine Zed config directory"))?
            .join("settings.json");

        Self::add_extension_to_settings(&settings_path)
    }
}
