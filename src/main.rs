use clap::Parser;
use color_eyre::{Result, eyre::ContextCompat};
use colored::Colorize;
use dialoguer::{Confirm, MultiSelect, Select, theme::ColorfulTheme};
use indicatif::ProgressBar;
use ini::{Ini, WriteOption};
use inkjet::{
    Highlighter, Language,
    formatter::Terminal,
    theme::{Theme, vendored},
};
use rand::Rng;
use rayon::prelude::*;
use termcolor::{ColorChoice, StandardStream};

mod editor_plugins;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The API key to use
    #[arg(short, long)]
    key: String,
}

fn generate_random_hostname() -> String {
    let mut rng = rand::rng();
    (0..6)
        .map(|_| rng.random_range(b'A'..=b'Z') as char)
        .collect::<String>()
}

fn build_config(api_key: &str, advanced: bool) -> Result<Ini> {
    let theme = ColorfulTheme::default();
    let mut conf = Ini::new();

    conf.with_section(Some("settings"))
        .set("api_url", "https://hackatime.hackclub.com/api/hackatime/v1")
        .set("api_key", api_key)
        .set("heartbeat_rate_limit_seconds", "30")
        .set("exclude_unknown_project", "true");

    if advanced {
        let hide_branch = Confirm::with_theme(&theme)
            .with_prompt("Hide branch names?")
            .default(false)
            .interact()?;

        let anonymize_hostname = Confirm::with_theme(&theme)
            .with_prompt("Anonymize your machine name?")
            .default(false)
            .interact()?;

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

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    println!("{}", "Welcome to Hackatime!\n".italic());

    let setup_options = vec!["Quick setup", "Advanced setup"];
    let setup_choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which setup mode would you like?")
        .items(&setup_options)
        .default(0)
        .interact()?;

    let is_advanced = setup_choice == 1;

    let conf = build_config(&cli.key, is_advanced)?;

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

    let write = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Should I write this to your WakaTime config?")
        .default(true)
        .interact()
        .unwrap();

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

    let all_editors = editor_plugins::all_editors();
    let installed_editors: Vec<_> = all_editors
        .into_par_iter()
        .filter(|e| e.is_installed())
        .collect();

    if installed_editors.is_empty() {
        println!("{}", "No supported editors found.".dimmed());
        return Ok(());
    }

    let editor_names: Vec<String> = installed_editors.iter().map(|e| e.name()).collect();
    let all_selected: Vec<bool> = vec![true; editor_names.len()];
    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("What editors should I install Hackatime to? (space to select/unselect)")
        .items(&editor_names)
        .defaults(&all_selected)
        .interact()?;

    if selections.is_empty() {
        println!("{}", "No editors selected.".dimmed());
        return Ok(());
    }

    let selected_editors: Vec<_> = selections
        .into_iter()
        .map(|i| &installed_editors[i])
        .collect();

    for editor in selected_editors {
        let name = editor.name();
        let pb = ProgressBar::new_spinner();
        pb.set_message(format!("Installing for {name}..."));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));

        match editor.install() {
            Ok(()) => pb.finish_with_message(format!("{} Installed for {}", "✔".green(), name)),
            Err(e) => pb.finish_with_message(format!("{} {} failed: {}", "✘".red(), name, e)),
        }
    }

    println!(
        "{}",
        "Done! You can now code in your editor to track your time.".bold()
    );
    println!(
        "Instructions for other editors: {}",
        "https://hackatime.hackclub.com/docs".underline().cyan()
    );
    println!(
        "{}",
        "hint: if time isn't being tracked, make sure you restart the editor first".dimmed()
    );

    Ok(())
}

fn print_ini(ini: &str) -> Result<()> {
    let mut highlighter = Highlighter::new();
    let theme = Theme::from_helix(vendored::AYU_DARK)?;

    let lines: Vec<&str> = ini.lines().collect();
    let max_width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let box_width = max_width + 2;

    println!("┌{}┐", "─".repeat(box_width));

    for line in lines {
        print!("│ ");
        let stream = StandardStream::stdout(ColorChoice::Always);
        let formatter = Terminal::new(theme.clone(), stream);
        highlighter.highlight_to_writer(Language::Ini, &formatter, line, &mut std::io::sink())?;
        let padding = max_width - line.chars().count();
        println!("{} │", " ".repeat(padding));
    }

    println!("└{}┘", "─".repeat(box_width));

    Ok(())
}
