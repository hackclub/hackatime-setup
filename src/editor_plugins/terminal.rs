use std::env::consts::{ARCH, OS};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use color_eyre::{Result, eyre::eyre};
use which::which;

use super::EditorPlugin;

const REPO: &str = "hackclub/terminal-wakatime";
const BINARY_NAME: &str = "terminal-wakatime";

#[derive(serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

pub struct TerminalWakaTime;

impl TerminalWakaTime {
    fn release_target() -> Result<(&'static str, &'static str)> {
        let os = match OS {
            "macos" => "darwin",
            "linux" => "linux",
            other => return Err(eyre!("Unsupported OS for terminal-wakatime: {other}")),
        };
        let arch = match ARCH {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            other => {
                return Err(eyre!(
                    "Unsupported architecture for terminal-wakatime: {other}"
                ));
            }
        };
        Ok((os, arch))
    }

    fn preferred_install_path() -> PathBuf {
        PathBuf::from("/usr/local/bin").join(BINARY_NAME)
    }

    fn fallback_install_dir() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|h| h.join(".wakatime"))
            .ok_or_else(|| eyre!("Could not determine home directory"))
    }

    fn fallback_install_path() -> Result<PathBuf> {
        Ok(Self::fallback_install_dir()?.join(BINARY_NAME))
    }

    fn existing_binary_path() -> Option<PathBuf> {
        let preferred = Self::preferred_install_path();
        if preferred.exists() {
            return Some(preferred);
        }

        if let Ok(fallback) = Self::fallback_install_path() {
            if fallback.exists() {
                return Some(fallback);
            }
        }

        which(BINARY_NAME).ok()
    }

    /// Whether setup installed the binary at `path`. A copy anywhere else, or
    /// a symlink (setup only writes regular files), was installed by something
    /// like Nix or Homebrew, which handles its updates.
    fn is_setup_install_path(path: &Path) -> bool {
        let setup_path = path == Self::preferred_install_path()
            || Self::fallback_install_path().is_ok_and(|fallback| path == fallback);
        setup_path && !path.is_symlink()
    }

    fn installed_version(path: &Path) -> Option<String> {
        // Prints "terminal-wakatime version v1.1.7"
        let output = std::process::Command::new(path)
            .arg("version")
            .output()
            .ok()?;
        let stdout = String::from_utf8(output.stdout).ok()?;
        stdout.split_whitespace().last().map(str::to_string)
    }

    fn install_latest() -> Result<PathBuf> {
        let client = reqwest::blocking::Client::new();
        let tag = Self::fetch_latest_tag(&client)?;
        let bytes = Self::download_binary(&client, &tag)?;
        Self::install_binary(&bytes)
    }

    /// Replaces `path` with the latest release if it is older. The binary is
    /// only ever replaced in place: a second copy elsewhere could end up behind
    /// the old one on PATH, so shells would keep running the old version.
    fn update_in_place(path: &Path) -> Result<()> {
        let client = reqwest::blocking::Client::new();
        let tag = Self::fetch_latest_tag(&client)?;
        if Self::installed_version(path).as_deref() == Some(tag.as_str()) {
            return Ok(());
        }

        let bytes = Self::download_binary(&client, &tag)?;
        Self::try_write_binary(path, &bytes)
    }

    fn has_supported_shell() -> bool {
        ["bash", "zsh", "fish"]
            .iter()
            .any(|shell| which(shell).is_ok())
    }

    fn fetch_latest_tag(client: &reqwest::blocking::Client) -> Result<String> {
        let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
        let response = client
            .get(&url)
            .header(reqwest::header::USER_AGENT, "hackatime-setup")
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .map_err(|e| eyre!("Failed to fetch latest terminal-wakatime release: {e}"))?;

        if !response.status().is_success() {
            return Err(eyre!(
                "Failed to fetch latest terminal-wakatime release (HTTP {})",
                response.status()
            ));
        }

        let release: GitHubRelease = response
            .json()
            .map_err(|e| eyre!("Failed to parse GitHub release response: {e}"))?;

        Ok(release.tag_name)
    }

    fn download_binary(client: &reqwest::blocking::Client, tag: &str) -> Result<Vec<u8>> {
        let (os, arch) = Self::release_target()?;
        let url =
            format!("https://github.com/{REPO}/releases/download/{tag}/{BINARY_NAME}-{os}-{arch}");

        let response = client
            .get(&url)
            .header(reqwest::header::USER_AGENT, "hackatime-setup")
            .send()
            .map_err(|e| eyre!("Failed to download terminal-wakatime: {e}"))?;

        if !response.status().is_success() {
            return Err(eyre!(
                "Failed to download terminal-wakatime (HTTP {})",
                response.status()
            ));
        }

        response
            .bytes()
            .map(|b| b.to_vec())
            .map_err(|e| eyre!("Failed to read terminal-wakatime download: {e}"))
    }

    fn install_binary(bytes: &[u8]) -> Result<PathBuf> {
        let preferred = Self::preferred_install_path();

        // Try preferred location first, fall back to ~/.wakatime
        let dest = if Self::try_write_binary(&preferred, bytes).is_ok() {
            preferred
        } else {
            let fallback_dir = Self::fallback_install_dir()?;
            fs::create_dir_all(&fallback_dir)
                .map_err(|e| eyre!("Failed to create {}: {e}", fallback_dir.display()))?;
            let fallback = fallback_dir.join(BINARY_NAME);
            Self::try_write_binary(&fallback, bytes)
                .map_err(|e| eyre!("Failed to install terminal-wakatime: {e}"))?;
            fallback
        };

        Ok(dest)
    }

    fn try_write_binary(path: &Path, bytes: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| eyre!("Failed to create {}: {e}", parent.display()))?;
        }

        // Write a uniquely named file beside the target and rename it into
        // place, so replacing a binary that is currently running doesn't fail
        // with "text file busy" and overlapping runs can't clash. The
        // temporary file is deleted if anything fails.
        let dir = path.parent().unwrap_or(Path::new("."));
        let write_err = |e: std::io::Error| eyre!("Failed to write {}: {e}", path.display());
        let mut tmp = tempfile::NamedTempFile::new_in(dir).map_err(write_err)?;
        tmp.write_all(bytes).map_err(write_err)?;
        Self::make_executable(tmp.path())?;
        tmp.persist(path).map_err(|e| write_err(e.error))?;
        Ok(())
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let perms = std::fs::Permissions::from_mode(0o755);
        fs::set_permissions(path, perms).map_err(|e| {
            eyre!(
                "Failed to set executable permissions on {}: {e}",
                path.display()
            )
        })
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &Path) -> Result<()> {
        Ok(())
    }

    fn shell_configs() -> Vec<(&'static str, PathBuf)> {
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return Vec::new(),
        };

        let mut configs = Vec::new();

        if which("bash").is_ok() {
            let rc = home.join(".bashrc");
            if rc.exists() {
                configs.push(("bash", rc));
            } else {
                configs.push(("bash", home.join(".bash_profile")));
            }
        }

        if which("zsh").is_ok() {
            configs.push(("zsh", home.join(".zshrc")));
        }

        if which("fish").is_ok() {
            let fish_config = dirs::config_dir()
                .unwrap_or_else(|| home.join(".config"))
                .join("fish/config.fish");
            configs.push(("fish", fish_config));
        }

        configs
    }

    fn configure_shell(shell: &str, config_path: &Path, needs_path: bool) -> Result<()> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = if config_path.exists() {
            fs::read_to_string(config_path)
                .map_err(|e| eyre!("Failed to read {}: {e}", config_path.display()))?
        } else {
            String::new()
        };

        let mut to_append = Vec::new();

        if needs_path {
            let path_line = match shell {
                "fish" => r#"set -gx PATH "$HOME/.wakatime" $PATH"#,
                _ => r#"export PATH="$HOME/.wakatime:$PATH""#,
            };
            if !contents.contains(path_line) {
                to_append.push(path_line);
            }
        }

        let init_line = match shell {
            "fish" => "terminal-wakatime init fish | source",
            _ => r#"eval "$(terminal-wakatime init)""#,
        };
        if !contents.contains(init_line) {
            to_append.push(init_line);
        }

        if to_append.is_empty() {
            return Ok(());
        }

        let mut new_contents = contents;
        if !new_contents.is_empty() && !new_contents.ends_with('\n') {
            new_contents.push('\n');
        }
        new_contents.push_str("\n# terminal-wakatime setup\n");
        new_contents.push_str(&to_append.join("\n"));
        new_contents.push('\n');

        fs::write(config_path, new_contents)
            .map_err(|e| eyre!("Failed to write {}: {e}", config_path.display()))
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

    fn install(&self) -> Result<Option<String>> {
        #[cfg(target_os = "windows")]
        {
            return Err(eyre!(
                "terminal-wakatime setup is not currently supported on Windows (requires bash, zsh, or fish)"
            ));
        }

        #[cfg(not(target_os = "windows"))]
        {
            if !Self::has_supported_shell() {
                return Err(eyre!("No supported shell found (bash, zsh, fish)"));
            }

            let mut warning = None;
            let binary_path = match Self::existing_binary_path() {
                Some(path) if !Self::is_setup_install_path(&path) => path,
                Some(path) => {
                    // Keep the working install if the update fails, for
                    // example when offline or when it isn't writable
                    if let Err(e) = Self::update_in_place(&path) {
                        warning = Some(format!("terminal-wakatime wasn't updated: {e}"));
                    }
                    path
                }
                None => Self::install_latest()?,
            };

            let needs_path = binary_path
                .parent()
                .and_then(|p| Self::fallback_install_dir().ok().map(|f| p == f))
                .unwrap_or(false);

            let shells = Self::shell_configs();
            for (shell, config_path) in &shells {
                Self::configure_shell(shell, config_path, needs_path)?;
            }

            Ok(warning)
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn dir_entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn write_binary_replaces_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(BINARY_NAME);
        fs::write(&path, b"old").unwrap();

        TerminalWakaTime::try_write_binary(&path, b"new").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"new");
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
        // No temporary files are left behind
        assert_eq!(dir_entries(dir.path()), vec![BINARY_NAME]);
    }

    #[test]
    fn write_binary_failure_keeps_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(BINARY_NAME);
        fs::write(&path, b"old").unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).unwrap();

        // Root can write to read-only directories, so this can't fail there
        let is_root = fs::write(dir.path().join("probe"), b"").is_ok();
        let result = TerminalWakaTime::try_write_binary(&path, b"new");
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
        if is_root {
            return;
        }

        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"old");
        assert_eq!(dir_entries(dir.path()), vec![BINARY_NAME]);
    }
}
