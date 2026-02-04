mod jetbrains;
mod utils;
mod vscode;
mod zed;

use color_eyre::Result;

pub use jetbrains::JetBrainsFamily;
pub use vscode::VsCodeFamily;
pub use zed::Zed;

pub trait EditorPlugin: Send + Sync {
    /// Human-readable name, e.g. "VS Code", "Cursor"
    fn name(&self) -> String;

    /// Detect whether this editor is installed
    fn is_installed(&self) -> bool;

    /// Install the WakaTime plugin for this editor
    fn install(&self) -> Result<()>;
}

pub fn all_editors() -> Vec<Box<dyn EditorPlugin>> {
    vec![
        // VS Code family
        Box::new(VsCodeFamily {
            name: "VS Code",
            config_subdir: ".vscode",
            cli_command: "code",
            macos_app_name: "Visual Studio Code",
            windows_app_folder: "Microsoft VS Code",
        }),
        Box::new(VsCodeFamily {
            name: "Cursor",
            config_subdir: ".cursor",
            cli_command: "cursor",
            macos_app_name: "Cursor",
            windows_app_folder: "cursor",
        }),
        Box::new(VsCodeFamily {
            name: "Windsurf",
            config_subdir: ".windsurf",
            cli_command: "windsurf",
            macos_app_name: "Windsurf",
            windows_app_folder: "windsurf",
        }),
        Box::new(VsCodeFamily {
            name: "Antigravity",
            config_subdir: ".antigravity",
            cli_command: "antigravity",
            macos_app_name: "Antigravity",
            windows_app_folder: "antigravity",
        }),
        Box::new(VsCodeFamily {
            name: "VSCodium",
            config_subdir: ".vscode-oss",
            cli_command: "codium",
            macos_app_name: "VSCodium",
            windows_app_folder: "VSCodium",
        }),
        Box::new(VsCodeFamily {
            name: "Trae",
            config_subdir: ".trae",
            cli_command: "trae",
            macos_app_name: "Trae",
            windows_app_folder: "Trae",
        }),
        // Zed
        Box::new(Zed),
        // JetBrains family
        Box::new(JetBrainsFamily {
            name: "IntelliJ IDEA",
            product_codes: &["IntelliJIdea", "IdeaIC"],
            cli_command: "idea",
            macos_app_names: &["IntelliJ IDEA", "IntelliJ IDEA CE"],
        }),
        Box::new(JetBrainsFamily {
            name: "PyCharm",
            product_codes: &["PyCharm", "PyCharmCE"],
            cli_command: "pycharm",
            macos_app_names: &["PyCharm", "PyCharm CE"],
        }),
        Box::new(JetBrainsFamily {
            name: "WebStorm",
            product_codes: &["WebStorm"],
            cli_command: "webstorm",
            macos_app_names: &["WebStorm"],
        }),
        Box::new(JetBrainsFamily {
            name: "GoLand",
            product_codes: &["GoLand"],
            cli_command: "goland",
            macos_app_names: &["GoLand"],
        }),
        Box::new(JetBrainsFamily {
            name: "RustRover",
            product_codes: &["RustRover"],
            cli_command: "rustrover",
            macos_app_names: &["RustRover"],
        }),
        Box::new(JetBrainsFamily {
            name: "RubyMine",
            product_codes: &["RubyMine"],
            cli_command: "rubymine",
            macos_app_names: &["RubyMine"],
        }),
        Box::new(JetBrainsFamily {
            name: "PhpStorm",
            product_codes: &["PhpStorm"],
            cli_command: "phpstorm",
            macos_app_names: &["PhpStorm"],
        }),
        Box::new(JetBrainsFamily {
            name: "CLion",
            product_codes: &["CLion"],
            cli_command: "clion",
            macos_app_names: &["CLion"],
        }),
        Box::new(JetBrainsFamily {
            name: "DataGrip",
            product_codes: &["DataGrip"],
            cli_command: "datagrip",
            macos_app_names: &["DataGrip"],
        }),
        Box::new(JetBrainsFamily {
            name: "Rider",
            product_codes: &["Rider"],
            cli_command: "rider",
            macos_app_names: &["Rider"],
        }),
        Box::new(JetBrainsFamily {
            name: "Android Studio",
            product_codes: &["AndroidStudio"],
            cli_command: "studio",
            macos_app_names: &["Android Studio"],
        }),
        Box::new(JetBrainsFamily {
            name: "AppCode",
            product_codes: &["AppCode"],
            cli_command: "appcode",
            macos_app_names: &["AppCode"],
        }),
    ]
}
