mod jetbrains;
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
            name: "IntelliJ IDEA Ultimate",
            product_code: "IntelliJIdea",
            cli_command: "idea",
            macos_app_name: "IntelliJ IDEA",
        }),
        Box::new(JetBrainsFamily {
            name: "IntelliJ IDEA Community",
            product_code: "IdeaIC",
            cli_command: "idea",
            macos_app_name: "IntelliJ IDEA CE",
        }),
        Box::new(JetBrainsFamily {
            name: "PyCharm Professional",
            product_code: "PyCharm",
            cli_command: "pycharm",
            macos_app_name: "PyCharm",
        }),
        Box::new(JetBrainsFamily {
            name: "PyCharm Community",
            product_code: "PyCharmCE",
            cli_command: "pycharm",
            macos_app_name: "PyCharm CE",
        }),
        Box::new(JetBrainsFamily {
            name: "WebStorm",
            product_code: "WebStorm",
            cli_command: "webstorm",
            macos_app_name: "WebStorm",
        }),
        Box::new(JetBrainsFamily {
            name: "GoLand",
            product_code: "GoLand",
            cli_command: "goland",
            macos_app_name: "GoLand",
        }),
        Box::new(JetBrainsFamily {
            name: "RustRover",
            product_code: "RustRover",
            cli_command: "rustrover",
            macos_app_name: "RustRover",
        }),
        Box::new(JetBrainsFamily {
            name: "RubyMine",
            product_code: "RubyMine",
            cli_command: "rubymine",
            macos_app_name: "RubyMine",
        }),
        Box::new(JetBrainsFamily {
            name: "PhpStorm",
            product_code: "PhpStorm",
            cli_command: "phpstorm",
            macos_app_name: "PhpStorm",
        }),
        Box::new(JetBrainsFamily {
            name: "CLion",
            product_code: "CLion",
            cli_command: "clion",
            macos_app_name: "CLion",
        }),
        Box::new(JetBrainsFamily {
            name: "DataGrip",
            product_code: "DataGrip",
            cli_command: "datagrip",
            macos_app_name: "DataGrip",
        }),
        Box::new(JetBrainsFamily {
            name: "Rider",
            product_code: "Rider",
            cli_command: "rider",
            macos_app_name: "Rider",
        }),
        Box::new(JetBrainsFamily {
            name: "Android Studio",
            product_code: "AndroidStudio",
            cli_command: "studio",
            macos_app_name: "Android Studio",
        }),
        Box::new(JetBrainsFamily {
            name: "AppCode",
            product_code: "AppCode",
            cli_command: "appcode",
            macos_app_name: "AppCode",
        }),
    ]
}
