//! Supported programming languages

use serde::{Deserialize, Serialize};

/// Supported programming languages for code analysis and manipulation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    Rust,
    TypeScript,
    JavaScript,
    Go,
    Java,
    CSharp,
    Ruby,
    RubySolargraph,
    PHP,
    Perl,
    PowerShell,
    Elixir,
    Terraform,
    Clojure,
    Swift,
    Bash,
    Vue,
    Cpp,
    C,
    Kotlin,
    Scala,
    Haskell,
    Erlang,
    OCaml,
    FSharp,
    Lua,
    R,
    Julia,
    Dart,
    ObjectiveC,
    Groovy,
    Zig,
    Nim,
    Crystal,
    Elm,
    PureScript,
    ReasonML,
    Solidity,
    YAML,
    TOML,
    JSON,
    XML,
    HTML,
    CSS,
    SCSS,
    Markdown,
}

impl Language {
    /// Get the display name for the language
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Python => "Python",
            Language::Rust => "Rust",
            Language::TypeScript => "TypeScript",
            Language::JavaScript => "JavaScript",
            Language::Go => "Go",
            Language::Java => "Java",
            Language::CSharp => "C#",
            Language::Ruby => "Ruby",
            Language::RubySolargraph => "Ruby (Solargraph)",
            Language::PHP => "PHP",
            Language::Perl => "Perl",
            Language::PowerShell => "PowerShell",
            Language::Elixir => "Elixir",
            Language::Terraform => "Terraform",
            Language::Clojure => "Clojure",
            Language::Swift => "Swift",
            Language::Bash => "Bash",
            Language::Vue => "Vue",
            Language::Cpp => "C++",
            Language::C => "C",
            Language::Kotlin => "Kotlin",
            Language::Scala => "Scala",
            Language::Haskell => "Haskell",
            Language::Erlang => "Erlang",
            Language::OCaml => "OCaml",
            Language::FSharp => "F#",
            Language::Lua => "Lua",
            Language::R => "R",
            Language::Julia => "Julia",
            Language::Dart => "Dart",
            Language::ObjectiveC => "Objective-C",
            Language::Groovy => "Groovy",
            Language::Zig => "Zig",
            Language::Nim => "Nim",
            Language::Crystal => "Crystal",
            Language::Elm => "Elm",
            Language::PureScript => "PureScript",
            Language::ReasonML => "ReasonML",
            Language::Solidity => "Solidity",
            Language::YAML => "YAML",
            Language::TOML => "TOML",
            Language::JSON => "JSON",
            Language::XML => "XML",
            Language::HTML => "HTML",
            Language::CSS => "CSS",
            Language::SCSS => "SCSS",
            Language::Markdown => "Markdown",
        }
    }

    /// Get common file extensions for the language
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::Python => &["py", "pyw", "pyi"],
            Language::Rust => &["rs"],
            Language::TypeScript => &["ts", "tsx"],
            Language::JavaScript => &["js", "jsx", "mjs", "cjs"],
            Language::Go => &["go"],
            Language::Java => &["java"],
            Language::CSharp => &["cs", "csx"],
            Language::Ruby | Language::RubySolargraph => &["rb", "rake"],
            Language::PHP => &["php", "phtml"],
            Language::Perl => &["pl", "pm"],
            Language::PowerShell => &["ps1", "psm1", "psd1"],
            Language::Elixir => &["ex", "exs"],
            Language::Terraform => &["tf", "tfvars"],
            Language::Clojure => &["clj", "cljs", "cljc", "edn"],
            Language::Swift => &["swift"],
            Language::Bash => &["sh", "bash"],
            Language::Vue => &["vue"],
            Language::Cpp => &["cpp", "cc", "cxx", "hpp", "hxx", "h++"],
            Language::C => &["c", "h"],
            Language::Kotlin => &["kt", "kts"],
            Language::Scala => &["scala", "sc"],
            Language::Haskell => &["hs", "lhs"],
            Language::Erlang => &["erl", "hrl"],
            Language::OCaml => &["ml", "mli"],
            Language::FSharp => &["fs", "fsi", "fsx"],
            Language::Lua => &["lua"],
            Language::R => &["r", "R"],
            Language::Julia => &["jl"],
            Language::Dart => &["dart"],
            Language::ObjectiveC => &["m", "mm"],
            Language::Groovy => &["groovy", "gradle"],
            Language::Zig => &["zig"],
            Language::Nim => &["nim"],
            Language::Crystal => &["cr"],
            Language::Elm => &["elm"],
            Language::PureScript => &["purs"],
            Language::ReasonML => &["re", "rei"],
            Language::Solidity => &["sol"],
            Language::YAML => &["yaml", "yml"],
            Language::TOML => &["toml"],
            Language::JSON => &["json", "jsonc"],
            Language::XML => &["xml"],
            Language::HTML => &["html", "htm"],
            Language::CSS => &["css"],
            Language::SCSS => &["scss", "sass"],
            Language::Markdown => &["md", "markdown"],
        }
    }

    /// Parse language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "py" | "pyw" | "pyi" => Some(Language::Python),
            "rs" => Some(Language::Rust),
            "ts" | "tsx" => Some(Language::TypeScript),
            "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
            "go" => Some(Language::Go),
            "java" => Some(Language::Java),
            "cs" | "csx" => Some(Language::CSharp),
            "rb" | "rake" => Some(Language::Ruby),
            "php" | "phtml" => Some(Language::PHP),
            "pl" | "pm" => Some(Language::Perl),
            "ps1" | "psm1" | "psd1" => Some(Language::PowerShell),
            "ex" | "exs" => Some(Language::Elixir),
            "tf" | "tfvars" => Some(Language::Terraform),
            "clj" | "cljs" | "cljc" | "edn" => Some(Language::Clojure),
            "swift" => Some(Language::Swift),
            "sh" | "bash" => Some(Language::Bash),
            "vue" => Some(Language::Vue),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "h++" => Some(Language::Cpp),
            "c" | "h" => Some(Language::C),
            "kt" | "kts" => Some(Language::Kotlin),
            "scala" | "sc" => Some(Language::Scala),
            "hs" | "lhs" => Some(Language::Haskell),
            "erl" | "hrl" => Some(Language::Erlang),
            "ml" | "mli" => Some(Language::OCaml),
            "fs" | "fsi" | "fsx" => Some(Language::FSharp),
            "lua" => Some(Language::Lua),
            "r" => Some(Language::R),
            "jl" => Some(Language::Julia),
            "dart" => Some(Language::Dart),
            "m" | "mm" => Some(Language::ObjectiveC),
            "groovy" | "gradle" => Some(Language::Groovy),
            "zig" => Some(Language::Zig),
            "nim" => Some(Language::Nim),
            "cr" => Some(Language::Crystal),
            "elm" => Some(Language::Elm),
            "purs" => Some(Language::PureScript),
            "re" | "rei" => Some(Language::ReasonML),
            "sol" => Some(Language::Solidity),
            "yaml" | "yml" => Some(Language::YAML),
            "toml" => Some(Language::TOML),
            "json" | "jsonc" => Some(Language::JSON),
            "xml" => Some(Language::XML),
            "html" | "htm" => Some(Language::HTML),
            "css" => Some(Language::CSS),
            "scss" | "sass" => Some(Language::SCSS),
            "md" | "markdown" => Some(Language::Markdown),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_extension() {
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("unknown"), None);
    }

    #[test]
    fn test_display_name() {
        assert_eq!(Language::Python.display_name(), "Python");
        assert_eq!(Language::CSharp.display_name(), "C#");
        assert_eq!(Language::TypeScript.display_name(), "TypeScript");
    }

    #[test]
    fn test_extensions() {
        assert!(Language::Python.extensions().contains(&"py"));
        assert!(Language::Rust.extensions().contains(&"rs"));
        assert!(Language::TypeScript.extensions().contains(&"ts"));
    }
}
