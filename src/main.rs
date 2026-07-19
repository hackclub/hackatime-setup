use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use color_eyre::{Result, eyre::ContextCompat};
use colored::Colorize;
use indicatif::ProgressBar;
use ini::{Ini, WriteOption};
use inkjet::{
    Highlighter, Language,
    formatter::Terminal,
    theme::{Theme, vendored},
};
use inquire::{Confirm, MultiSelect, Select};
use rand::Rng;
use rayon::prelude::*;
use reqwest::blocking::Client;
use serde::Serialize;
use termcolor::{ColorChoice, StandardStream};
use uuid::Uuid;

use crate::editor_plugins::EditorPlugin;

mod editor_plugins;

const DEFAULT_API_URL: &str = "https://hackatime.hackclub.com/api/hackatime/v1";

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The API key to use
    #[arg(short, long)]
    key: String,

    /// The API URL to use
    #[arg(long, default_value = DEFAULT_API_URL)]
    api_url: String,

    /// Skip all prompts and use defaults (quick setup, install for all detected editors)
    #[arg(short, long)]
    yes: bool,
}

#[derive(Serialize)]
struct Heartbeat {
    #[serde(rename = "type")]
    kind: String,
    time: u64,
    entity: String,
    language: String,
}

fn generate_random_hostname() -> String {
    let mut rng = rand::rng();
    (0..6)
        .map(|_| rng.random_range(b'A'..=b'Z') as char)
        .collect::<String>()
}

fn send_test_heartbeat(api_key: &str, api_url: &str) -> Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let heartbeat = Heartbeat {
        kind: "file".to_string(),
        time: timestamp,
        entity: "test.txt".to_string(),
        language: "Text".to_string(),
    };

    let client = Client::new();
    let response = client
        .post(format!("{api_url}/users/current/heartbeats"))
        .bearer_auth(api_key)
        .json(&vec![heartbeat])
        .send()?;

    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        let body = response.text().unwrap_or_default();
        Err(color_eyre::eyre::eyre!(
            "Test heartbeat failed ({}): {}",
            status,
            body
        ))
    }
}

fn build_config(api_key: &str, api_url: &str, advanced: bool) -> Result<Ini> {
    let mut conf = Ini::new();

    conf.with_section(Some("settings"))
        .set("api_url", api_url)
        .set("api_key", api_key)
        .set("heartbeat_rate_limit_seconds", "30")
        .set("exclude_unknown_project", "true");

    if advanced {
        let hide_branch = Confirm::new("Hide branch names?")
            .with_default(false)
            .prompt()?;

        let anonymize_hostname = Confirm::new("Anonymize your machine name?")
            .with_default(false)
            .prompt()?;

        if hide_branch {
            conf.with_section(Some("settings"))
                .set("hide_branch_names", "true");
        }

        if anonymize_hostname {
            let hostname = generate_random_hostname();
            conf.with_section(Some("settings"))
                .set("hostname", &hostname);
            println!("{} {}", "Generated hostname:".dimmed(), hostname.cyan());
        }
    }

    Ok(conf)
}

fn validate_api_key(key: &str) -> Result<(), String> {
    let uuid = Uuid::try_parse(key)
        .map_err(|_| "API key must be a valid UUID. Did you copy the command incorrectly?")?;
    if uuid.get_version_num() != 4 {
        return Err("API key must be a valid UUIDv4".to_string());
    }
    Ok(())
}

fn current_utc_rfc3339() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let days = timestamp.div_euclid(86_400);
    let seconds_of_day = timestamp.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_unix_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year, month, day)
}

fn seed_ai_logs_last_parsed_at() -> Result<()> {
    let internal_config_path = dirs::home_dir()
        .wrap_err("Could not find home directory")?
        .join(".wakatime")
        .join("wakatime-internal.cfg");

    if let Some(parent) = internal_config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut conf = if internal_config_path.exists() {
        Ini::load_from_file(&internal_config_path).unwrap_or_else(|_| Ini::new())
    } else {
        Ini::new()
    };

    let now = current_utc_rfc3339();
    conf.with_section(Some("internal"))
        .set("ai_logs_last_parsed_at", now);

    let write_opt = WriteOption {
        kv_separator: " = ",
        ..Default::default()
    };
    conf.write_to_file_opt(&internal_config_path, write_opt)?;

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    if let Err(e) = validate_api_key(&cli.key) {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }

    println!("{}", "Welcome to Hackatime!\n".italic());

    let is_advanced = if cli.yes {
        false
    } else {
        let setup_options = vec!["Quick setup", "Advanced setup"];
        let setup_choice = Select::new("Which setup mode would you like?", setup_options)
            .with_starting_cursor(0)
            .prompt()?;
        setup_choice == "Advanced setup"
    };

    let conf = build_config(&cli.key, &cli.api_url, is_advanced)?;

    let write_opt = WriteOption {
        kv_separator: " = ",
        ..Default::default()
    };

    let mut config_string = Vec::new();
    conf.write_to_opt(&mut config_string, write_opt.clone())?;
    config_string.extend_from_slice(b"\n# help with config: https://github.com/wakatime/wakatime-cli/blob/develop/USAGE.md#ini-config-file");
    let generated_config = String::from_utf8(config_string)?;

    println!(
        "\nHere's the {} file I'm planning to write:\n",
        "~/.wakatime.cfg".green()
    );
    print_ini(&generated_config)?;
    println!();

    let write = if cli.yes {
        true
    } else {
        Confirm::new("Should I write this to your WakaTime config?")
            .with_default(true)
            .prompt()?
    };

    if !write {
        eprintln!("{}", "Understood, exiting now.".dimmed());
        return Ok(());
    }

    let config_path = dirs::home_dir()
        .wrap_err("Could not find home directory")?
        .join(".wakatime.cfg");

    conf.write_to_file_opt(&config_path, write_opt)?;
    println!(
        "{} {}\n",
        "✔".green().bold(),
        format!("Config written to {}", config_path.display()).green()
    );

    if let Err(e) = seed_ai_logs_last_parsed_at() {
        eprintln!(
            "{} Could not update WakaTime internal config to skip initial AI activity backfill: {}",
            "Warning:".yellow(),
            e
        );
    }

    let all_editors = editor_plugins::all_editors();
    let installed_editors: Vec<_> = all_editors
        .into_par_iter()
        .filter(|e| e.is_installed())
        .collect();

    if installed_editors.is_empty() {
        println!("{}", "No supported editors found.".dimmed());
    } else {
        let editor_names: Vec<String> = installed_editors.iter().map(|e| e.name()).collect();
        let selections = if cli.yes {
            editor_names.clone()
        } else {
            let all_selected: Vec<usize> = (0..editor_names.len()).collect();
            MultiSelect::new("What editors should I install Hackatime to?", editor_names)
                .with_default(&all_selected)
                .prompt()?
        };

        if !selections.is_empty() {
            let selected_editors: Vec<_> = installed_editors
                .iter()
                .filter(|e| selections.contains(&e.name()))
                .collect();
            install_plugins(selected_editors);
        } else {
            println!(
                "\n{}",
                "No editors selected, skipping editor plugin installation.".dimmed()
            );
        }
    }

    println!(
        "Instructions for other editors: {}",
        "https://hackatime.hackclub.com/docs".underline().cyan()
    );
    println!(
        "{} {}",
        "hint: if time isn't being tracked, make sure you restart the editor first. need help?"
            .dimmed(),
        "http://forms.hackclub.com/setup"
            .dimmed()
            .cyan()
            .underline()
    );

    if let Err(e) = send_test_heartbeat(&cli.key, &cli.api_url) {
        eprintln!("{} {}", "Warning:".yellow(), e);
    }

    Ok(())
}

fn install_plugins(selected_editors: Vec<&Box<dyn EditorPlugin>>) {
    for editor in selected_editors {
        let name = editor.name();
        let pb = ProgressBar::new_spinner();
        pb.set_message(format!("Installing for {name}..."));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));

        match editor.install() {
            Ok(warning) => {
                pb.finish_with_message(format!("{} Installed for {}", "✔".green(), name));
                if let Some(msg) = warning {
                    eprintln!("  {} {}", "▲".yellow(), msg);
                }
            }
            Err(e) => pb.finish_with_message(format!("{} {} failed: {}", "✘".red(), name, e)),
        }
    }

    println!(
        "\n{}",
        "Done! You can now code in your editor to track your time.".bold()
    );
}

fn print_ini(ini: &str) -> Result<()> {
    let mut highlighter = Highlighter::new();
    let theme = Theme::from_helix(vendored::AYU_DARK)?;

    let term_width = terminal_size::terminal_size().map_or(80, |(w, _)| w.0 as usize);
    let content_width = term_width.saturating_sub(4).max(20);
    let border_width = content_width + 2;

    println!("┌{}┐", "─".repeat(border_width));

    for line in ini.lines() {
        let line_len = line.chars().count();
        if line_len <= content_width {
            print!("│ ");
            let stream = StandardStream::stdout(ColorChoice::Always);
            let formatter = Terminal::new(theme.clone(), stream);
            highlighter.highlight_to_writer(
                Language::Ini,
                &formatter,
                line,
                &mut std::io::sink(),
            )?;
            let padding = content_width - line_len;
            println!("{} │", " ".repeat(padding));
        } else {
            let mut remaining = line;
            while !remaining.is_empty() {
                let chunk: String = remaining.chars().take(content_width).collect();
                let chunk_len = chunk.chars().count();
                remaining = &remaining[chunk.len()..];

                print!("│ ");
                let stream = StandardStream::stdout(ColorChoice::Always);
                let formatter = Terminal::new(theme.clone(), stream);
                highlighter.highlight_to_writer(
                    Language::Ini,
                    &formatter,
                    &chunk,
                    &mut std::io::sink(),
                )?;
                let padding = content_width - chunk_len;
                println!("{} │", " ".repeat(padding));
            }
        }
    }

    println!("└{}┘", "─".repeat(border_width));

    Ok(())
}
