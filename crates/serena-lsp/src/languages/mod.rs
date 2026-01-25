//! Language server configurations
//!
//! Defines configurations for various language servers, including
//! command names, arguments, and file extension mappings.

use serena_config::Language;

/// Configuration for a language server
#[derive(Debug, Clone)]
pub struct LanguageServerConfig {
    /// Command to execute the language server
    pub command: String,

    /// Command-line arguments
    pub args: Vec<String>,

    /// File extensions this language server handles
    pub file_extensions: Vec<&'static str>,
}

/// Get the language server configuration for a specific language
///
/// # Arguments
/// * `language` - The programming language
///
/// # Returns
/// The language server configuration, or an error if the language is not supported
pub fn get_config(language: Language) -> anyhow::Result<LanguageServerConfig> {
    let config = match language {
        Language::Rust => LanguageServerConfig {
            command: "rust-analyzer".to_string(),
            args: vec![],
            file_extensions: vec!["rs"],
        },

        Language::Python => LanguageServerConfig {
            command: "pyright-langserver".to_string(),
            args: vec!["--stdio".to_string()],
            file_extensions: vec!["py", "pyw", "pyi"],
        },

        Language::TypeScript | Language::JavaScript => LanguageServerConfig {
            command: "typescript-language-server".to_string(),
            args: vec!["--stdio".to_string()],
            file_extensions: vec!["ts", "tsx", "js", "jsx", "mjs", "cjs"],
        },

        Language::Go => LanguageServerConfig {
            command: "gopls".to_string(),
            args: vec![],
            file_extensions: vec!["go"],
        },

        Language::Java => LanguageServerConfig {
            command: "jdtls".to_string(),
            args: vec![],
            file_extensions: vec!["java"],
        },

        Language::CSharp => LanguageServerConfig {
            command: "csharp-ls".to_string(),
            args: vec![],
            file_extensions: vec!["cs", "csx"],
        },

        Language::Ruby => LanguageServerConfig {
            command: "ruby-lsp".to_string(),
            args: vec![],
            file_extensions: vec!["rb", "rake"],
        },

        Language::RubySolargraph => LanguageServerConfig {
            command: "solargraph".to_string(),
            args: vec!["stdio".to_string()],
            file_extensions: vec!["rb", "rake"],
        },

        Language::PHP => LanguageServerConfig {
            command: "intelephense".to_string(),
            args: vec!["--stdio".to_string()],
            file_extensions: vec!["php", "phtml"],
        },

        Language::Perl => LanguageServerConfig {
            command: "pls".to_string(),
            args: vec![],
            file_extensions: vec!["pl", "pm"],
        },

        Language::PowerShell => LanguageServerConfig {
            command: "pwsh".to_string(),
            args: vec![
                "-NoLogo".to_string(),
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "PowerShellEditorServices".to_string(),
            ],
            file_extensions: vec!["ps1", "psm1", "psd1"],
        },

        Language::Elixir => LanguageServerConfig {
            command: "elixir-ls".to_string(),
            args: vec![],
            file_extensions: vec!["ex", "exs"],
        },

        Language::Terraform => LanguageServerConfig {
            command: "terraform-ls".to_string(),
            args: vec!["serve".to_string()],
            file_extensions: vec!["tf", "tfvars"],
        },

        Language::Clojure => LanguageServerConfig {
            command: "clojure-lsp".to_string(),
            args: vec![],
            file_extensions: vec!["clj", "cljs", "cljc", "edn"],
        },

        Language::Swift => LanguageServerConfig {
            command: "sourcekit-lsp".to_string(),
            args: vec![],
            file_extensions: vec!["swift"],
        },

        Language::Bash => LanguageServerConfig {
            command: "bash-language-server".to_string(),
            args: vec!["start".to_string()],
            file_extensions: vec!["sh", "bash"],
        },

        Language::Vue => LanguageServerConfig {
            command: "vue-language-server".to_string(),
            args: vec!["--stdio".to_string()],
            file_extensions: vec!["vue"],
        },

        Language::Cpp => LanguageServerConfig {
            command: "clangd".to_string(),
            args: vec![],
            file_extensions: vec!["cpp", "cc", "cxx", "hpp", "hxx", "h++"],
        },

        Language::C => LanguageServerConfig {
            command: "clangd".to_string(),
            args: vec![],
            file_extensions: vec!["c", "h"],
        },

        Language::Kotlin => LanguageServerConfig {
            command: "kotlin-language-server".to_string(),
            args: vec![],
            file_extensions: vec!["kt", "kts"],
        },

        Language::Scala => LanguageServerConfig {
            command: "metals".to_string(),
            args: vec![],
            file_extensions: vec!["scala", "sc"],
        },

        Language::Haskell => LanguageServerConfig {
            command: "haskell-language-server-wrapper".to_string(),
            args: vec!["--lsp".to_string()],
            file_extensions: vec!["hs", "lhs"],
        },

        Language::Erlang => LanguageServerConfig {
            command: "erlang_ls".to_string(),
            args: vec![],
            file_extensions: vec!["erl", "hrl"],
        },

        Language::FSharp => LanguageServerConfig {
            command: "fsautocomplete".to_string(),
            args: vec!["--background-service-enabled".to_string()],
            file_extensions: vec!["fs", "fsi", "fsx"],
        },

        Language::Lua => LanguageServerConfig {
            command: "lua-language-server".to_string(),
            args: vec![],
            file_extensions: vec!["lua"],
        },

        Language::R => LanguageServerConfig {
            command: "R".to_string(),
            args: vec![
                "--slave".to_string(),
                "-e".to_string(),
                "languageserver::run()".to_string(),
            ],
            file_extensions: vec!["r", "R"],
        },

        Language::Julia => LanguageServerConfig {
            command: "julia".to_string(),
            args: vec![
                "--startup-file=no".to_string(),
                "--history-file=no".to_string(),
                "-e".to_string(),
                "using LanguageServer; runserver()".to_string(),
            ],
            file_extensions: vec!["jl"],
        },

        Language::Dart => LanguageServerConfig {
            command: "dart".to_string(),
            args: vec!["language-server".to_string()],
            file_extensions: vec!["dart"],
        },

        Language::Groovy => LanguageServerConfig {
            command: "groovy-language-server".to_string(),
            args: vec![],
            file_extensions: vec!["groovy", "gradle"],
        },

        Language::Zig => LanguageServerConfig {
            command: "zls".to_string(),
            args: vec![],
            file_extensions: vec!["zig"],
        },

        Language::YAML => LanguageServerConfig {
            command: "yaml-language-server".to_string(),
            args: vec!["--stdio".to_string()],
            file_extensions: vec!["yaml", "yml"],
        },

        Language::TOML => LanguageServerConfig {
            command: "taplo".to_string(),
            args: vec!["lsp".to_string(), "stdio".to_string()],
            file_extensions: vec!["toml"],
        },

        Language::Markdown => LanguageServerConfig {
            command: "marksman".to_string(),
            args: vec!["server".to_string()],
            file_extensions: vec!["md", "markdown"],
        },

        _ => {
            return Err(anyhow::anyhow!(
                "Language server configuration not available for {:?}",
                language
            ));
        }
    };

    Ok(config)
}

/// Get the language for a file based on its extension
///
/// # Arguments
/// * `file_path` - Path to the file
///
/// # Returns
/// The detected language, or `None` if the extension is not recognized
pub fn detect_language(file_path: &str) -> Option<Language> {
    let extension = std::path::Path::new(file_path).extension()?.to_str()?;

    Language::from_extension(extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_config() {
        let config = get_config(Language::Rust).unwrap();
        assert_eq!(config.command, "rust-analyzer");
        assert!(config.file_extensions.contains(&"rs"));
    }

    #[test]
    fn test_python_config() {
        let config = get_config(Language::Python).unwrap();
        assert_eq!(config.command, "pyright-langserver");
        assert!(config.file_extensions.contains(&"py"));
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("test.rs"), Some(Language::Rust));
        assert_eq!(detect_language("test.py"), Some(Language::Python));
        assert_eq!(detect_language("test.ts"), Some(Language::TypeScript));
        assert_eq!(detect_language("test.go"), Some(Language::Go));
        assert_eq!(detect_language("test.unknown"), None);
    }

    #[test]
    fn test_typescript_javascript_share_config() {
        let ts_config = get_config(Language::TypeScript).unwrap();
        let js_config = get_config(Language::JavaScript).unwrap();
        assert_eq!(ts_config.command, js_config.command);
    }
}
