use std::env::consts::{ARCH, OS};
use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::{Result, eyre::eyre};
use which::which;

use super::EditorPlugin;

const REPO: &str = "hackclub/terminal-wakatime";
const BINARY_NAME: &str = "terminal-wakatime";
const NU_SETUP_MARKER: &str = "# terminal-wakatime setup (nu)";
const NU_PATH_LINE: &str = r#"if (($env.PATH | any {|p| $p == ($env.HOME | path join ".wakatime")}) == false) { $env.PATH = ($env.PATH | prepend ($env.HOME | path join ".wakatime")) }"#;
const NU_HOOKS_BLOCK: &str = r#"$env.config = ($env.config
| upsert hooks.pre_execution (
    (($env.config | get -o hooks.pre_execution) | default [])
    | append {||
        $env.__TERMINAL_WAKATIME_COMMAND = (commandline)
        $env.__TERMINAL_WAKATIME_PWD = $env.PWD
    }
)
| upsert hooks.pre_prompt (
    (($env.config | get -o hooks.pre_prompt) | default [])
    | append {||
        if ('__TERMINAL_WAKATIME_COMMAND' in $env) and ('__TERMINAL_WAKATIME_PWD' in $env) {
            let command = $env.__TERMINAL_WAKATIME_COMMAND
            let pwd = $env.__TERMINAL_WAKATIME_PWD
            hide-env __TERMINAL_WAKATIME_COMMAND
            hide-env __TERMINAL_WAKATIME_PWD

            let duration_ms = (($env | get -o CMD_DURATION_MS) | default 0)
            let duration = (($duration_ms | into int) / 1000)

            if $duration >= 1 {
                ^terminal-wakatime track --command $command --duration $duration --pwd $pwd out+err> /dev/null
            }
        }
    }
))"#;

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

        if let Ok(fallback) = Self::fallback_install_path()
            && fallback.exists()
        {
            return Some(fallback);
        }

        which(BINARY_NAME).ok()
    }

    fn has_supported_shell() -> bool {
        ["bash", "zsh", "fish", "nu"]
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

        Self::make_executable(&dest)?;
        Ok(dest)
    }

    fn try_write_binary(path: &Path, bytes: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| eyre!("Failed to create {}: {e}", parent.display()))?;
        }
        fs::write(path, bytes).map_err(|e| eyre!("Failed to write {}: {e}", path.display()))
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

        if which("nu").is_ok() {
            let nu_config = dirs::config_dir()
                .unwrap_or_else(|| home.join(".config"))
                .join("nushell/config.nu");
            configs.push(("nu", nu_config));
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

        if shell == "nu" {
            return Self::configure_nushell(config_path, contents, needs_path);
        }

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

    fn configure_nushell(config_path: &Path, contents: String, needs_path: bool) -> Result<()> {
        let Some(new_contents) = Self::build_nushell_config(&contents, needs_path) else {
            return Ok(());
        };

        fs::write(config_path, new_contents)
            .map_err(|e| eyre!("Failed to write {}: {e}", config_path.display()))
    }

    fn build_nushell_config(contents: &str, needs_path: bool) -> Option<String> {
        if contents.contains(NU_SETUP_MARKER) {
            return None;
        }

        let mut new_contents = contents.to_string();
        if !new_contents.is_empty() && !new_contents.ends_with('\n') {
            new_contents.push('\n');
        }
        new_contents.push('\n');
        new_contents.push_str(NU_SETUP_MARKER);
        new_contents.push('\n');

        if needs_path {
            new_contents.push_str(NU_PATH_LINE);
            new_contents.push('\n');
            new_contents.push('\n');
        }

        new_contents.push_str(NU_HOOKS_BLOCK);
        new_contents.push('\n');

        Some(new_contents)
    }
}

#[cfg(test)]
mod tests {
    use super::{NU_HOOKS_BLOCK, NU_PATH_LINE, NU_SETUP_MARKER, TerminalWakaTime};

    #[test]
    fn build_nushell_config_is_idempotent_when_marker_exists() {
        let existing = format!("before\n{NU_SETUP_MARKER}\nafter\n");
        assert!(TerminalWakaTime::build_nushell_config(&existing, true).is_none());
    }

    #[test]
    fn build_nushell_config_includes_path_and_blank_line_when_needed() {
        let generated = TerminalWakaTime::build_nushell_config("", true)
            .expect("config should be generated when marker is absent");

        let expected_gap = format!("{NU_PATH_LINE}\n\n{NU_HOOKS_BLOCK}");
        assert!(generated.contains(&expected_gap));
    }

    #[test]
    fn build_nushell_config_omits_path_when_not_needed() {
        let generated = TerminalWakaTime::build_nushell_config("", false)
            .expect("config should be generated when marker is absent");

        assert!(!generated.contains(NU_PATH_LINE));
        assert!(generated.contains(NU_HOOKS_BLOCK));
    }
}

impl EditorPlugin for TerminalWakaTime {
    fn name(&self) -> String {
        "Terminal (bash/zsh/fish/nu)".to_string()
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
                "terminal-wakatime setup is not currently supported on Windows (requires bash, zsh, fish, or nu)"
            ));
        }

        #[cfg(not(target_os = "windows"))]
        {
            if !Self::has_supported_shell() {
                return Err(eyre!("No supported shell found (bash, zsh, fish, nu)"));
            }

            let binary_path = if let Some(path) = Self::existing_binary_path() {
                path
            } else {
                let client = reqwest::blocking::Client::new();
                let tag = Self::fetch_latest_tag(&client)?;
                let bytes = Self::download_binary(&client, &tag)?;
                Self::install_binary(&bytes)?
            };

            let needs_path = binary_path
                .parent()
                .and_then(|p| Self::fallback_install_dir().ok().map(|f| p == f))
                .unwrap_or(false);

            let shells = Self::shell_configs();
            for (shell, config_path) in &shells {
                Self::configure_shell(shell, config_path, needs_path)?;
            }

            Ok(None)
        }
    }
}
