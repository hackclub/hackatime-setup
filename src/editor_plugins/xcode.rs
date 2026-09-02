use std::path::PathBuf;

use color_eyre::{Result, eyre::eyre};

use super::EditorPlugin;

const DOWNLOAD_URL: &str = "https://github.com/hackclub/xcode-hackatime/releases/latest/download/xcode-hackatime-darwin-universal.zip";

pub struct Xcode;

impl Xcode {
    #[cfg(target_os = "macos")]
    fn agent_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".wakatime/xcode-hackatime")
    }
}

impl EditorPlugin for Xcode {
    fn name(&self) -> String {
        "Xcode".to_string()
    }

    fn is_installed(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            PathBuf::from("/Applications/Xcode.app").exists()
                || std::process::Command::new("xcrun")
                    .arg("--version")
                    .output()
                    .is_ok_and(|o| o.status.success())
        }

        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    fn install(&self) -> Result<Option<String>> {
        #[cfg(not(target_os = "macos"))]
        {
            return Err(eyre!("Xcode is only supported on macOS"));
        }

        #[cfg(target_os = "macos")]
        {
            use std::fs;
            use std::process::Command;

            let tmp_dir =
                tempfile::tempdir().map_err(|e| eyre!("Failed to create temp directory: {}", e))?;
            let zip_path = tmp_dir.path().join("xcode-hackatime.zip");

            let client = reqwest::blocking::Client::new();
            let response = client
                .get(DOWNLOAD_URL)
                .send()
                .map_err(|e| eyre!("Failed to download xcode-hackatime: {}", e))?;

            if !response.status().is_success() {
                return Err(eyre!(
                    "Failed to download xcode-hackatime (HTTP {})",
                    response.status()
                ));
            }

            let bytes = response
                .bytes()
                .map_err(|e| eyre!("Failed to read download: {}", e))?;
            fs::write(&zip_path, &bytes).map_err(|e| eyre!("Failed to write zip file: {}", e))?;

            let status = Command::new("ditto")
                .args([
                    "-xk",
                    &zip_path.to_string_lossy(),
                    &tmp_dir.path().to_string_lossy(),
                ])
                .output()
                .map_err(|e| eyre!("Failed to unzip: {}", e))?;

            if !status.status.success() {
                return Err(eyre!(
                    "Failed to unzip xcode-hackatime: {}",
                    String::from_utf8_lossy(&status.stderr)
                ));
            }

            let binary = tmp_dir.path().join("xcode-hackatime");
            if !binary.exists() {
                return Err(eyre!("xcode-hackatime not found in downloaded archive"));
            }

            // make sure it's actually from The Hack Foundation (us)
            const HACK_FOUNDATION_TEAM_ID: &str = "P6PV2R9443";
            let requirement = format!(
                "-R=anchor apple generic and certificate leaf[subject.OU] = \"{}\"",
                HACK_FOUNDATION_TEAM_ID
            );
            let verify = Command::new("codesign")
                .args([
                    "--verify",
                    "--strict",
                    &requirement,
                    &binary.to_string_lossy(),
                ])
                .output()
                .map_err(|e| eyre!("Failed to run codesign: {}", e))?;
            if !verify.status.success() {
                return Err(eyre!(
                    "downloaded xcode-hackatime failed code-signature verification; not installing it"
                ));
            }

            let status = Command::new("chmod")
                .args(["+x", &binary.to_string_lossy()])
                .output()
                .map_err(|e| eyre!("Failed to chmod xcode-hackatime: {}", e))?;
            if !status.status.success() {
                return Err(eyre!(
                    "Failed to chmod xcode-hackatime: {}",
                    String::from_utf8_lossy(&status.stderr)
                ));
            }

            // `install` copies the binary to ~/.wakatime, registers a launchd
            // agent, starts it, and downloads wakatime-cli if it's missing.
            let status = Command::new(&binary)
                .arg("install")
                .output()
                .map_err(|e| eyre!("Failed to run xcode-hackatime install: {}", e))?;
            if !status.status.success() {
                return Err(eyre!(
                    "xcode-hackatime install failed: {}{}",
                    String::from_utf8_lossy(&status.stdout),
                    String::from_utf8_lossy(&status.stderr)
                ));
            }

            if !Self::agent_path().exists() {
                return Err(eyre!("xcode-hackatime agent was not installed"));
            }

            let warning = String::from(
                "One manual step: System Settings → Privacy & Security → Accessibility → enable \"xcode-hackatime\" (a prompt should appear). Tracking will work once that's done!",
            );
            Ok(Some(warning))
        }
    }
}
