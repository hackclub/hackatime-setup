use clap::Parser;
use color_eyre::{Result, eyre::ContextCompat};
use colored::Colorize;
use dialoguer::{Confirm, MultiSelect, theme::ColorfulTheme};
use indicatif::ProgressBar;
use inkjet::{
    Highlighter, Language,
    formatter::Terminal,
    theme::{Theme, vendored},
};
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

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    println!(
        "{} {}\n",
        "Welcome to Hackatime,".bold().blue(),
        "Mahad!".bold()
    );

    println!(
        "Here's the {} file I'm planning to write:\n",
        "~/.wakatime.cfg".green()
    );

    let generated_config: String =
        include_str!("./config.ini.template").replace("{{key}}", &cli.key);
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

    std::fs::write(&config_path, &generated_config)?;
    println!(
        "{} {}",
        "✔".green().bold(),
        format!("Config written to {}", config_path.display()).green()
    );

    println!("\n{}", "Scanning for installed editors...".dimmed());
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
        .with_prompt("What editors should I install Hackatime to?")
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
