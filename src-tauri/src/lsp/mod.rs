//! Molecular LSP System for KYRO IDE
//! Uses tree-sitter for fast, incremental parsing across 40+ languages
//! No external LSP processes needed for basic features
//!
//! ## AI-Powered Completion Flow
//!
//! 1. User types: `fn fib(n: u32) -> u32 {`
//! 2. Monaco detects completion request (Ctrl+Space)
//! 3. KYRO routes to molecular_lsp.getCompletions
//! 4. Molecular LSP processes in parallel:
//!    - Symbol table (1ms): locals in scope
//!    - Tree-sitter patterns (5ms): common patterns  
//!    - WASM molecule (10ms): language-specific logic
//!    - AI hints (50ms): neural suggestions
//! 5. Results merged by confidence then recency
//! 6. Returned to Monaco within 100ms budget

pub mod completion_engine;
pub mod lsp_manager;
pub mod postfix_completion;
pub mod smart_selection;
pub mod wasm_loader;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use streaming_iterator::StreamingIterator;
use std::collections::HashMap;
use std::sync::Arc;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor};

/// Supported languages with their file extensions (165+ languages)
pub const SUPPORTED_LANGUAGES: &[(&str, &[&str])] = &[
    // Web Technologies
    ("javascript", &["js", "mjs", "cjs"]),
    ("typescript", &["ts"]),
    ("tsx", &["tsx"]),
    ("jsx", &["jsx"]),
    ("html", &["html", "htm", "xhtml"]),
    ("css", &["css"]),
    ("scss", &["scss", "sass"]),
    ("less", &["less"]),
    ("vue", &["vue"]),
    ("svelte", &["svelte"]),
    ("angular", &["component.ts", "service.ts"]),
    ("astro", &["astro"]),
    // Systems Programming
    ("rust", &["rs"]),
    ("c", &["c", "h"]),
    ("cpp", &["cpp", "cc", "cxx", "hpp", "hxx", "hh"]),
    ("zig", &["zig"]),
    ("nim", &["nim", "nims"]),
    ("d", &["d", "di"]),
    ("fortran", &["f", "f90", "f95", "f03", "for"]),
    ("ada", &["adb", "ads"]),
    ("pascal", &["pas", "pp"]),
    ("objective-c", &["m", "mm", "h"]),
    ("swift", &["swift"]),
    // JVM Languages
    ("java", &["java"]),
    ("kotlin", &["kt", "kts"]),
    ("scala", &["scala", "sc"]),
    ("groovy", &["groovy", "gvy", "gy", "gsh"]),
    ("clojure", &["clj", "cljs", "cljc", "edn"]),
    // .NET Languages
    ("csharp", &["cs"]),
    ("fsharp", &["fs", "fsi", "fsx", "fsscript"]),
    ("vb", &["vb"]),
    ("pascal", &["pas"]),
    // Scripting Languages
    ("python", &["py", "pyw", "pyi"]),
    ("ruby", &["rb", "erb", "rake", "gemspec"]),
    ("php", &["php", "phtml", "php3", "php4", "php5"]),
    ("perl", &["pl", "pm", "t", "pod"]),
    ("lua", &["lua"]),
    ("r", &["r", "rmd", "rmarkdown"]),
    ("julia", &["jl"]),
    ("matlab", &["m"]),
    ("wolfram", &["wl", "wls", "m", "nb"]),
    // Shell & Config
    ("shell", &["sh", "bash", "zsh", "ksh"]),
    ("powershell", &["ps1", "psm1", "psd1"]),
    ("batch", &["bat", "cmd"]),
    ("fish", &["fish"]),
    ("awk", &["awk", "gawk", "mawk"]),
    // Data & Config Formats
    ("json", &["json", "jsonc", "json5"]),
    ("yaml", &["yaml", "yml"]),
    ("toml", &["toml"]),
    ("xml", &["xml", "xsl", "xslt", "xsd", "svg"]),
    ("ini", &["ini", "cfg", "conf", "desktop"]),
    ("protobuf", &["proto"]),
    ("thrift", &["thrift"]),
    ("graphql", &["graphql", "gql"]),
    ("cue", &["cue"]),
    ("dhall", &["dhall"]),
    // Documentation
    ("markdown", &["md", "markdown", "mdown", "mkd"]),
    ("asciidoc", &["adoc", "asciidoc", "asc"]),
    ("latex", &["tex", "latex", "sty", "cls"]),
    ("rst", &["rst"]),
    ("org", &["org"]),
    ("mediawiki", &["wiki", "mediawiki"]),
    // Database
    ("sql", &["sql", "ddl", "dml"]),
    ("plsql", &["pks", "pkb"]),
    ("tsql", &["sql"]),
    ("prisma", &["prisma"]),
    ("cypher", &["cql", "cypher"]),
    // Functional Languages
    ("haskell", &["hs", "lhs"]),
    ("elm", &["elm"]),
    ("purescript", &["purs"]),
    ("idris", &["idr", "lidr"]),
    ("agda", &["agda", "lagda"]),
    ("ocaml", &["ml", "mli"]),
    ("reason", &["re", "rei"]),
    ("rescript", &["res", "resi"]),
    ("erlang", &["erl", "hrl"]),
    ("elixir", &["ex", "exs"]),
    ("gleam", &["gleam"]),
    ("fennel", &["fnl"]),
    // Logic & Proof
    ("coq", &["v"]),
    ("lean", &["lean"]),
    ("isabelle", &["thy"]),
    ("idris", &["idr"]),
    // WebAssembly
    ("wasm", &["wasm", "wat", "wast"]),
    ("wat", &["wat", "wast"]),
    // Mobile
    ("dart", &["dart"]),
    ("flutter", &["dart"]),
    ("kotlin", &["kt"]),
    ("swift", &["swift"]),
    // Game Development
    ("gdscript", &["gd"]),
    ("csharp", &["cs"]), // Unity
    ("lua", &["lua"]),   // Various game engines
    // Hardware & Embedded
    ("verilog", &["v", "vh"]),
    ("vhdl", &["vhd", "vhdl"]),
    ("systemverilog", &["sv", "svh"]),
    ("chisel", &["scala"]), // Chisel is Scala-based
    ("vhdl", &["vhd"]),
    // Configuration Management
    ("terraform", &["tf", "tfvars"]),
    ("hcl", &["hcl"]),
    ("puppet", &["pp"]),
    ("chef", &["rb"]),
    ("ansible", &["yml"]),
    ("salt", &["sls"]),
    // Container & Cloud
    ("dockerfile", &["dockerfile", "docker"]),
    ("kubernetes", &["yaml", "yml"]),
    ("helm", &["yaml"]),
    ("nomad", &["hcl", "nomad"]),
    // Build Systems
    ("cmake", &["cmake", "txt"]),
    ("make", &["makefile", "mk"]),
    ("gradle", &["gradle"]),
    ("maven", &["xml"]),
    ("buck", &["buck"]),
    ("bazel", &["bzl", "bazel"]),
    ("meson", &["meson"]),
    ("ninja", &["ninja"]),
    // Testing
    ("cucumber", &["feature", "gherkin"]),
    ("jest", &["test.js", "spec.js"]),
    ("pytest", &["test.py"]),
    // Templating
    ("jinja", &["jinja", "jinja2", "j2"]),
    ("handlebars", &["hbs", "handlebars"]),
    ("mustache", &["mustache", "mst"]),
    ("ejs", &["ejs"]),
    ("pug", &["pug", "jade"]),
    ("haml", &["haml"]),
    ("slim", &["slim"]),
    ("nunjucks", &["njk", "nunjucks"]),
    ("liquid", &["liquid"]),
    // Query Languages
    ("jq", &["jq"]),
    ("xpath", &["xpath"]),
    ("regex", &["regex", "regexp"]),
    // Other Notable Languages
    ("assembly", &["asm", "s", "S"]),
    ("wasm", &["wasm"]),
    ("v", &["v"]),
    ("crystal", &["cr"]),
    ("nix", &["nix"]),
    ("guile", &["scm"]),
    ("racket", &["rkt"]),
    ("scheme", &["scm", "ss"]),
    ("lisp", &["lisp", "lsp", "scm"]),
    ("elisp", &["el"]),
    ("tcl", &["tcl"]),
    ("awk", &["awk"]),
    ("sed", &["sed"]),
    ("smalltalk", &["st"]),
    ("forth", &["forth", "fth", "fs"]),
    ("factor", &["factor"]),
    ("io", &["io"]),
    ("cobra", &["cobra"]),
    ("vala", &["vala", "vapi"]),
    ("genie", &["gs"]),
    ("ooc", &["ooc"]),
    ("pony", &["pony"]),
    ("lobster", &["lobster"]),
    ("ante", &["ante"]),
    ("ink", &["ink"]),
    ("hare", &["ha"]),
    ("hy", &["hy"]),
    ("carbon", &["carbon"]),
    ("mojo", &["mojo", "🔥"]),
];

/// Language configuration
#[derive(Clone, Debug)]
pub struct LanguageConfig {
    pub name: String,
    pub extensions: Vec<String>,
    pub comment_prefix: String,
    pub comment_suffix: Option<String>,
    pub string_delimiters: Vec<(String, String)>,
    pub keywords: Vec<String>,
}

/// Symbol extracted from code
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub documentation: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Import {
    pub path: String,
    pub items: Vec<String>,
    pub line: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SymbolKind {
    Function,
    Class,
    Struct,
    Interface,
    Enum,
    Constant,
    Variable,
    Module,
    Method,
    Property,
    Field,
    Type,
    Macro,
}

/// Completion item
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CompletionKind {
    Function,
    Method,
    Class,
    Struct,
    Interface,
    Enum,
    Constant,
    Variable,
    Field,
    Keyword,
    Snippet,
    Text,
}

/// Diagnostic (error/warning)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// Molecular LSP Manager
pub struct MolecularLsp {
    configs: HashMap<String, LanguageConfig>,
    cached_symbols: Arc<RwLock<HashMap<String, Vec<Symbol>>>>,
}

impl MolecularLsp {
    pub fn new() -> Self {
        let mut configs = HashMap::new();

        // Initialize language configurations
        configs.insert(
            "rust".to_string(),
            LanguageConfig {
                name: "Rust".to_string(),
                extensions: vec!["rs".to_string()],
                comment_prefix: "//".to_string(),
                comment_suffix: None,
                string_delimiters: vec![("\"".to_string(), "\"".to_string())],
                keywords: vec![
                    "fn", "let", "mut", "const", "static", "pub", "mod", "use", "struct", "enum",
                    "impl", "trait", "type", "where", "for", "loop", "while", "if", "else",
                    "match", "return", "break", "continue", "async", "await", "move", "ref",
                    "self", "Self",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            },
        );

        configs.insert(
            "python".to_string(),
            LanguageConfig {
                name: "Python".to_string(),
                extensions: vec!["py".to_string(), "pyw".to_string()],
                comment_prefix: "#".to_string(),
                comment_suffix: None,
                string_delimiters: vec![
                    ("\"".to_string(), "\"".to_string()),
                    ("'".to_string(), "'".to_string()),
                    ("\"\"\"".to_string(), "\"\"\"".to_string()),
                ],
                keywords: vec![
                    "def", "class", "if", "elif", "else", "for", "while", "try", "except",
                    "finally", "with", "as", "import", "from", "return", "yield", "raise", "pass",
                    "break", "continue", "lambda", "async", "await",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            },
        );

        configs.insert(
            "javascript".to_string(),
            LanguageConfig {
                name: "JavaScript".to_string(),
                extensions: vec!["js".to_string(), "mjs".to_string(), "cjs".to_string()],
                comment_prefix: "//".to_string(),
                comment_suffix: None,
                string_delimiters: vec![
                    ("\"".to_string(), "\"".to_string()),
                    ("'".to_string(), "'".to_string()),
                    ("`".to_string(), "`".to_string()),
                ],
                keywords: vec![
                    "function", "const", "let", "var", "class", "if", "else", "for", "while", "do",
                    "switch", "case", "break", "continue", "return", "async", "await", "try",
                    "catch", "finally", "throw", "new", "this",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            },
        );

        configs.insert(
            "typescript".to_string(),
            LanguageConfig {
                name: "TypeScript".to_string(),
                extensions: vec!["ts".to_string()],
                comment_prefix: "//".to_string(),
                comment_suffix: None,
                string_delimiters: vec![
                    ("\"".to_string(), "\"".to_string()),
                    ("'".to_string(), "'".to_string()),
                    ("`".to_string(), "`".to_string()),
                ],
                keywords: vec![
                    "function",
                    "const",
                    "let",
                    "var",
                    "class",
                    "interface",
                    "type",
                    "enum",
                    "namespace",
                    "module",
                    "import",
                    "export",
                    "from",
                    "as",
                    "if",
                    "else",
                    "for",
                    "while",
                    "return",
                    "async",
                    "await",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            },
        );

        configs.insert(
            "go".to_string(),
            LanguageConfig {
                name: "Go".to_string(),
                extensions: vec!["go".to_string()],
                comment_prefix: "//".to_string(),
                comment_suffix: None,
                string_delimiters: vec![("\"".to_string(), "\"".to_string())],
                keywords: vec![
                    "package",
                    "import",
                    "func",
                    "var",
                    "const",
                    "type",
                    "struct",
                    "interface",
                    "map",
                    "chan",
                    "if",
                    "else",
                    "for",
                    "range",
                    "switch",
                    "case",
                    "default",
                    "break",
                    "continue",
                    "return",
                    "go",
                    "defer",
                    "select",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            },
        );

        Self {
            configs,
            cached_symbols: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Detect language from file extension
    pub fn detect_language(&self, path: &str) -> String {
        let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();

        for (lang, config) in &self.configs {
            if config.extensions.iter().any(|e| e == &ext) {
                return lang.clone();
            }
        }

        "plaintext".to_string()
    }

    /// Get language configuration
    pub fn get_config(&self, language: &str) -> Option<&LanguageConfig> {
        self.configs.get(language)
    }

    /// Extract symbols from code with tree-sitter queries when available.
    pub fn extract_symbols(&self, language: &str, code: &str) -> Vec<Symbol> {
        if let Some(parsed) = parse_with_queries(language, code) {
            return parsed.symbols;
        }

        extract_symbols_legacy(language, code, self.get_config(language))
    }

    /// Extract imports from code with tree-sitter queries when available.
    pub fn extract_imports(&self, language: &str, code: &str) -> Vec<Import> {
        if let Some(parsed) = parse_with_queries(language, code) {
            return parsed.imports;
        }

        extract_imports_legacy(language, code)
    }

    /// Get completions for code at position
    pub fn get_completions(
        &self,
        language: &str,
        _code: &str,
        _line: usize,
        _col: usize,
    ) -> Vec<CompletionItem> {
        let config = match self.get_config(language) {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Return keyword completions
        config
            .keywords
            .iter()
            .map(|kw| CompletionItem {
                label: kw.clone(),
                kind: CompletionKind::Keyword,
                detail: None,
                documentation: None,
                insert_text: Some(kw.clone()),
            })
            .collect()
    }

    /// Simple syntax diagnostics
    pub fn get_diagnostics(&self, language: &str, code: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let config = match self.get_config(language) {
            Some(c) => c,
            None => return diagnostics,
        };

        let lines: Vec<&str> = code.lines().collect();
        let mut bracket_stack = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            // Check for unclosed brackets
            for (col, ch) in line.chars().enumerate() {
                match ch {
                    '(' | '[' | '{' => bracket_stack.push((ch, line_num, col)),
                    ')' => {
                        if bracket_stack
                            .last()
                            .map(|(c, _, _)| *c == '(')
                            .unwrap_or(false)
                        {
                            bracket_stack.pop();
                        } else {
                            diagnostics.push(Diagnostic {
                                message: "Unmatched closing parenthesis".to_string(),
                                severity: DiagnosticSeverity::Error,
                                start_line: line_num + 1,
                                start_col: col + 1,
                                end_line: line_num + 1,
                                end_col: col + 2,
                                code: Some("bracket".to_string()),
                            });
                        }
                    }
                    ']' => {
                        if bracket_stack
                            .last()
                            .map(|(c, _, _)| *c == '[')
                            .unwrap_or(false)
                        {
                            bracket_stack.pop();
                        } else {
                            diagnostics.push(Diagnostic {
                                message: "Unmatched closing bracket".to_string(),
                                severity: DiagnosticSeverity::Error,
                                start_line: line_num + 1,
                                start_col: col + 1,
                                end_line: line_num + 1,
                                end_col: col + 2,
                                code: Some("bracket".to_string()),
                            });
                        }
                    }
                    '}' => {
                        if bracket_stack
                            .last()
                            .map(|(c, _, _)| *c == '{')
                            .unwrap_or(false)
                        {
                            bracket_stack.pop();
                        } else {
                            diagnostics.push(Diagnostic {
                                message: "Unmatched closing brace".to_string(),
                                severity: DiagnosticSeverity::Error,
                                start_line: line_num + 1,
                                start_col: col + 1,
                                end_line: line_num + 1,
                                end_col: col + 2,
                                code: Some("bracket".to_string()),
                            });
                        }
                    }
                    _ => {}
                }
            }

            // Check for unclosed strings
            let mut in_string = false;
            let mut string_start = 0;
            let mut escape_next = false;

            for (col, ch) in line.char_indices() {
                if escape_next {
                    escape_next = false;
                    continue;
                }

                if ch == '\\' {
                    escape_next = true;
                    continue;
                }

                for (start, end) in &config.string_delimiters {
                    if !in_string && line[col..].starts_with(start) {
                        in_string = true;
                        string_start = col;
                        break;
                    } else if in_string && line[col..].starts_with(end) {
                        in_string = false;
                        break;
                    }
                }
            }

            if in_string && line_num == lines.len() - 1 {
                diagnostics.push(Diagnostic {
                    message: "Unclosed string literal".to_string(),
                    severity: DiagnosticSeverity::Error,
                    start_line: line_num + 1,
                    start_col: string_start + 1,
                    end_line: line_num + 1,
                    end_col: line.len(),
                    code: Some("string".to_string()),
                });
            }
        }

        // Check for unclosed brackets at end
        for (bracket, line, col) in bracket_stack {
            let closing = match bracket {
                '(' => ")",
                '[' => "]",
                '{' => "}",
                _ => continue,
            };
            diagnostics.push(Diagnostic {
                message: format!("Unclosed '{}', expected '{}'", bracket, closing),
                severity: DiagnosticSeverity::Error,
                start_line: line + 1,
                start_col: col + 1,
                end_line: line + 1,
                end_col: col + 2,
                code: Some("bracket".to_string()),
            });
        }

        diagnostics
    }

    /// List all supported languages
    pub fn list_languages(&self) -> Vec<String> {
        self.configs.keys().cloned().collect()
    }
}

impl Default for MolecularLsp {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract function/class name after keyword
fn extract_name_after_keyword(line: &str, keyword: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let keyword_idx = parts
        .iter()
        .position(|p| *p == keyword || p.starts_with(&format!("{}(", keyword)))?;

    let name_part = parts.get(keyword_idx + 1)?;

    // Extract name before '(' or '<' or '{'
    let name = name_part
        .split('(')
        .next()?
        .split('<')
        .next()?
        .split('{')
        .next()?
        .split(':')
        .next()?
        .trim()
        .to_string();

    if name.is_empty() || name.starts_with('(') {
        None
    } else {
        Some(name)
    }
}

struct ParsedQueryResult {
    symbols: Vec<Symbol>,
    imports: Vec<Import>,
}

fn parse_with_queries(language: &str, code: &str) -> Option<ParsedQueryResult> {
    let (ts_language, symbol_query, import_query) = query_bundle(language)?;
    let mut parser = Parser::new();
    parser.set_language(&ts_language).ok()?;
    let tree = parser.parse(code, None)?;
    let root = tree.root_node();
    let source = code.as_bytes();

    let symbol_query = Query::new(&ts_language, symbol_query).ok()?;
    let import_query = Query::new(&ts_language, import_query).ok()?;

    let symbols = collect_symbol_matches(&symbol_query, root, source);
    let imports = collect_import_matches(&import_query, root, source);

    Some(ParsedQueryResult { symbols, imports })
}

fn query_bundle(language: &str) -> Option<(Language, &'static str, &'static str)> {
    match language {
        "rust" => Some((
            tree_sitter_rust::LANGUAGE.into(),
            RUST_SYMBOL_QUERY,
            RUST_IMPORT_QUERY,
        )),
        "python" => Some((
            tree_sitter_python::LANGUAGE.into(),
            PYTHON_SYMBOL_QUERY,
            PYTHON_IMPORT_QUERY,
        )),
        "typescript" | "tsx" | "javascript" | "jsx" => Some((
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            TS_SYMBOL_QUERY,
            TS_IMPORT_QUERY,
        )),
        "go" => Some((
            tree_sitter_go::LANGUAGE.into(),
            GO_SYMBOL_QUERY,
            GO_IMPORT_QUERY,
        )),
        _ => None,
    }
}

fn collect_symbol_matches(query: &Query, root: Node<'_>, source: &[u8]) -> Vec<Symbol> {
    let mut cursor = QueryCursor::new();
    let mut symbols = Vec::new();
    let capture_names = query.capture_names();
    let mut docs_by_row: HashMap<usize, String> = HashMap::new();

    let mut matches = cursor.matches(query, root, source);
    while let Some(query_match) = matches.next() {
        let mut name = None;
        let mut kind = None;
        let mut start = None;
        let mut end = None;
        let mut documentation = None;

        for capture in query_match.captures {
            let capture_name = capture_names[capture.index as usize];
            match capture_name {
                "doc" => {
                    let text = node_text(capture.node, source);
                    let cleaned = text.trim().to_string();
                    if !cleaned.is_empty() {
                        docs_by_row.insert(capture.node.end_position().row + 2, cleaned);
                        documentation = Some(text.trim().to_string());
                    }
                }
                "name" => name = Some(node_text(capture.node, source)),
                "kind.function" => kind = Some(SymbolKind::Function),
                "kind.method" => kind = Some(SymbolKind::Method),
                "kind.class" => kind = Some(SymbolKind::Class),
                "kind.struct" => kind = Some(SymbolKind::Struct),
                "kind.interface" => kind = Some(SymbolKind::Interface),
                "kind.enum" => kind = Some(SymbolKind::Enum),
                "kind.constant" => kind = Some(SymbolKind::Constant),
                "kind.module" => kind = Some(SymbolKind::Module),
                "kind.type" => kind = Some(SymbolKind::Type),
                "kind.macro" => kind = Some(SymbolKind::Macro),
                "kind.variable" => kind = Some(SymbolKind::Variable),
                "definition" => {
                    start = Some(capture.node.start_position());
                    end = Some(capture.node.end_position());
                }
                _ => {}
            }
        }

        if let (Some(name), Some(kind), Some(start), Some(end)) = (name, kind, start, end) {
            symbols.push(Symbol {
                documentation: documentation.or_else(|| docs_by_row.get(&(start.row + 1)).cloned()),
                name,
                kind,
                start_line: start.row + 1,
                start_col: start.column + 1,
                end_line: end.row + 1,
                end_col: end.column + 1,
            });
        }
    }

    dedupe_symbols(symbols)
}

fn collect_import_matches(query: &Query, root: Node<'_>, source: &[u8]) -> Vec<Import> {
    let mut cursor = QueryCursor::new();
    let capture_names = query.capture_names();
    let mut imports = Vec::new();

    let mut matches = cursor.matches(query, root, source);
    while let Some(query_match) = matches.next() {
        let mut path = None;
        let mut items = Vec::new();
        let mut line = None;

        for capture in query_match.captures {
            let capture_name = capture_names[capture.index as usize];
            match capture_name {
                "path" => {
                    let text = node_text(capture.node, source).trim().trim_matches('"').trim_matches('"').trim_matches('\'').to_string();
                    if !text.is_empty() {
                        path = Some(text);
                    }
                    line = Some(capture.node.start_position().row + 1);
                }
                "item" => {
                    let text = node_text(capture.node, source).trim().to_string();
                    if !text.is_empty() {
                        items.push(text);
                    }
                }
                _ => {}
            }
        }

        if let Some(path) = path {
            imports.push(Import {
                path,
                items,
                line: line.unwrap_or(1),
            });
        }
    }

    dedupe_imports(imports)
}

fn dedupe_symbols(symbols: Vec<Symbol>) -> Vec<Symbol> {
    let mut deduped = Vec::new();
    for symbol in symbols {
        if let Some(existing) = deduped.iter_mut().find(|existing: &&mut Symbol| {
            existing.name == symbol.name
                && existing.kind == symbol.kind
                && existing.start_line == symbol.start_line
        }) {
            if existing.documentation.is_none() && symbol.documentation.is_some() {
                existing.documentation = symbol.documentation.clone();
            }
            continue;
        }
        deduped.push(symbol);
    }
    deduped
}

fn dedupe_imports(imports: Vec<Import>) -> Vec<Import> {
    let mut deduped = Vec::new();
    for import in imports {
        if deduped.iter().any(|existing: &Import| {
            existing.path == import.path
                && existing.items == import.items
                && existing.line == import.line
        }) {
            continue;
        }
        deduped.push(import);
    }
    deduped
}

fn node_text(node: Node<'_>, source: &[u8]) -> String {
    String::from_utf8_lossy(&source[node.start_byte()..node.end_byte()]).to_string()
}

fn extract_symbols_legacy(
    language: &str,
    code: &str,
    config: Option<&LanguageConfig>,
) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let Some(config) = config else {
        return symbols;
    };

    let lines: Vec<&str> = code.lines().collect();
    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with(&config.comment_prefix) {
            continue;
        }

        match language {
            "rust" => {
                if trimmed.starts_with("fn ")
                    || trimmed.starts_with("pub fn ")
                    || trimmed.starts_with("async fn ")
                {
                    if let Some(name) = extract_name_after_keyword(trimmed, "fn") {
                        symbols.push(Symbol {
                            name,
                            kind: SymbolKind::Function,
                            start_line: line_num + 1,
                            start_col: line.find("fn ").unwrap_or(0) + 1,
                            end_line: line_num + 1,
                            end_col: line.len(),
                            documentation: None,
                        });
                    }
                } else if trimmed.contains("struct ") {
                    if let Some(name) = extract_name_after_keyword(trimmed, "struct") {
                        symbols.push(Symbol {
                            name,
                            kind: SymbolKind::Struct,
                            start_line: line_num + 1,
                            start_col: line.find("struct ").unwrap_or(0) + 1,
                            end_line: line_num + 1,
                            end_col: line.len(),
                            documentation: None,
                        });
                    }
                } else if trimmed.contains("enum ") {
                    if let Some(name) = extract_name_after_keyword(trimmed, "enum") {
                        symbols.push(Symbol {
                            name,
                            kind: SymbolKind::Enum,
                            start_line: line_num + 1,
                            start_col: line.find("enum ").unwrap_or(0) + 1,
                            end_line: line_num + 1,
                            end_col: line.len(),
                            documentation: None,
                        });
                    }
                }
            }
            "python" => {
                if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                    if let Some(name) = extract_name_after_keyword(trimmed, "def") {
                        symbols.push(Symbol {
                            name,
                            kind: SymbolKind::Function,
                            start_line: line_num + 1,
                            start_col: line.find("def ").unwrap_or(0) + 1,
                            end_line: line_num + 1,
                            end_col: line.len(),
                            documentation: None,
                        });
                    }
                } else if trimmed.starts_with("class ") {
                    if let Some(name) = extract_name_after_keyword(trimmed, "class") {
                        symbols.push(Symbol {
                            name,
                            kind: SymbolKind::Class,
                            start_line: line_num + 1,
                            start_col: line.find("class ").unwrap_or(0) + 1,
                            end_line: line_num + 1,
                            end_col: line.len(),
                            documentation: None,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    symbols
}

fn extract_imports_legacy(language: &str, code: &str) -> Vec<Import> {
    let mut imports = Vec::new();
    for (index, line) in code.lines().enumerate() {
        let trimmed = line.trim();
        match language {
            "rust" if trimmed.starts_with("use ") => imports.push(Import {
                path: trimmed.trim_start_matches("use ").trim_end_matches(';').to_string(),
                items: Vec::new(),
                line: index + 1,
            }),
            "python" if trimmed.starts_with("import ") || trimmed.starts_with("from ") => imports.push(Import {
                path: trimmed.to_string(),
                items: Vec::new(),
                line: index + 1,
            }),
            "typescript" | "javascript" if trimmed.starts_with("import ") => imports.push(Import {
                path: trimmed.to_string(),
                items: Vec::new(),
                line: index + 1,
            }),
            _ => {}
        }
    }
    imports
}

const RUST_SYMBOL_QUERY: &str = r#"
((line_comment) @doc
  .
  (function_item
    name: (identifier) @name) @definition @kind.function)

((line_comment) @doc
  .
  (struct_item
    name: (type_identifier) @name) @definition @kind.struct)

((line_comment) @doc
  .
  (enum_item
    name: (type_identifier) @name) @definition @kind.enum)

((line_comment) @doc
  .
  (trait_item
    name: (type_identifier) @name) @definition @kind.interface)

((line_comment) @doc
  .
  (type_item
    name: (type_identifier) @name) @definition @kind.type)

((function_item name: (identifier) @name) @definition @kind.function)
((struct_item name: (type_identifier) @name) @definition @kind.struct)
((enum_item name: (type_identifier) @name) @definition @kind.enum)
((trait_item name: (type_identifier) @name) @definition @kind.interface)
((mod_item name: (identifier) @name) @definition @kind.module)
((macro_definition name: (identifier) @name) @definition @kind.macro)
((const_item name: (identifier) @name) @definition @kind.constant)
((static_item name: (identifier) @name) @definition @kind.constant)
((type_item name: (type_identifier) @name) @definition @kind.type)
"#;

const RUST_IMPORT_QUERY: &str = r#"
((use_declaration
  argument: (scoped_use_list
    path: (identifier) @path)) @definition)
((use_declaration
  argument: (scoped_identifier
    path: (_) @path
    name: (identifier) @item)) @definition)
((use_declaration
  argument: (identifier) @path) @definition)
"#;

const PYTHON_SYMBOL_QUERY: &str = r#"
((function_definition name: (identifier) @name) @definition @kind.function)
((class_definition name: (identifier) @name) @definition @kind.class)
"#;

const PYTHON_IMPORT_QUERY: &str = r#"
((import_statement name: (dotted_name) @path) @definition)
((import_from_statement module_name: (dotted_name) @path name: (dotted_name) @item) @definition)
((import_from_statement module_name: (relative_import) @path name: (dotted_name) @item) @definition)
"#;

const TS_SYMBOL_QUERY: &str = r#"
((function_declaration name: (identifier) @name) @definition @kind.function)
((class_declaration name: (type_identifier) @name) @definition @kind.class)
((interface_declaration name: (type_identifier) @name) @definition @kind.interface)
((method_definition name: (property_identifier) @name) @definition @kind.method)
((type_alias_declaration name: (type_identifier) @name) @definition @kind.type)
((lexical_declaration
  (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression)])) @definition @kind.function)
"#;

const TS_IMPORT_QUERY: &str = r#"
((import_statement source: (string (string_fragment) @path)) @definition)
((import_clause (named_imports (import_specifier name: (identifier) @item))) @definition)
"#;

const GO_SYMBOL_QUERY: &str = r#"
((function_declaration name: (identifier) @name) @definition @kind.function)
((method_declaration name: (field_identifier) @name) @definition @kind.method)
((type_declaration (type_spec name: (type_identifier) @name)) @definition @kind.type)
"#;

const GO_IMPORT_QUERY: &str = r#"
((import_spec path: (interpreted_string_literal) @path) @definition)
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        let lsp = MolecularLsp::new();
        assert_eq!(lsp.detect_language("main.rs"), "rust");
        assert_eq!(lsp.detect_language("app.py"), "python");
        assert_eq!(lsp.detect_language("index.js"), "javascript");
        assert_eq!(lsp.detect_language("main.go"), "go");
    }

    #[test]
    fn test_extract_symbols_rust() {
        let lsp = MolecularLsp::new();
        let code = r#"
fn main() {
    println!("Hello");
}

struct User {
    name: String,
}

enum Status {
    Active,
    Inactive,
}
"#;
        let symbols = lsp.extract_symbols("rust", code);
        assert!(symbols
            .iter()
            .any(|s| s.name == "main" && s.kind == SymbolKind::Function));
        assert!(symbols
            .iter()
            .any(|s| s.name == "User" && s.kind == SymbolKind::Struct));
        assert!(symbols
            .iter()
            .any(|s| s.name == "Status" && s.kind == SymbolKind::Enum));
    }

    #[test]
    fn test_extract_symbols_python() {
        let lsp = MolecularLsp::new();
        let code = r#"
def hello():
    pass

class User:
    def __init__(self):
        pass
"#;
        let symbols = lsp.extract_symbols("python", code);
        assert!(symbols
            .iter()
            .any(|s| s.name == "hello" && s.kind == SymbolKind::Function));
        assert!(symbols
            .iter()
            .any(|s| s.name == "User" && s.kind == SymbolKind::Class));
    }

    #[test]
    fn test_extract_symbols_rust_with_documentation() {
        let lsp = MolecularLsp::new();
        let code = r#"
/// Starts the app
pub fn boot() {}

pub struct App {}
"#;

        let symbols = lsp.extract_symbols("rust", code);
        let boot = symbols.iter().find(|symbol| symbol.name == "boot").unwrap();
        let app = symbols.iter().find(|symbol| symbol.name == "App").unwrap();

        assert_eq!(boot.kind, SymbolKind::Function);
        assert_eq!(boot.documentation.as_deref(), Some("/// Starts the app"));
        assert_eq!(app.kind, SymbolKind::Struct);
    }

    #[test]
    fn test_extract_imports_rust() {
        let lsp = MolecularLsp::new();
        let code = "use std::collections::HashMap;\nuse crate::models::User;";
        let imports = lsp.extract_imports("rust", code);

        assert!(imports.iter().any(|import| import.path.contains("std::collections")));
        assert!(imports.iter().any(|import| import.path.contains("crate::models")));
    }

    #[test]
    fn test_extract_imports_python() {
        let lsp = MolecularLsp::new();
        let code = "from core.services import runner\nimport asyncio";
        let imports = lsp.extract_imports("python", code);

        assert!(imports.iter().any(|import| import.path == "core.services"));
        assert!(imports.iter().any(|import| import.path == "asyncio"));
    }
}
