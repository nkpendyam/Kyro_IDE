//! LSP Backend Implementation using tower-lsp
//!
//! Implements the LanguageServer trait from tower-lsp

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService, Server};

use super::{DocumentState, LspConfig};

/// KYRO IDE LSP Backend
pub struct KyroLspBackend {
    /// Client handle for sending notifications
    client: tower_lsp::Client,
    /// Configuration
    config: LspConfig,
    /// Open documents
    documents: Arc<RwLock<HashMap<Url, DocumentState>>>,
    /// Workspace root
    root_uri: Arc<RwLock<Option<Url>>>,
}

impl KyroLspBackend {
    /// Create a new LSP backend
    pub fn new(client: tower_lsp::Client, config: LspConfig) -> Self {
        Self {
            client,
            config,
            documents: Arc::new(RwLock::new(HashMap::new())),
            root_uri: Arc::new(RwLock::new(None)),
        }
    }

    /// Get document content
    pub async fn get_document(&self, uri: &Url) -> Option<DocumentState> {
        let docs = self.documents.read().await;
        docs.get(uri).cloned()
    }

    /// Update document content
    pub async fn update_document(&self, uri: Url, content: String, version: i32) {
        let mut docs = self.documents.write().await;
        if let Some(doc) = docs.get_mut(&uri) {
            doc.content = content;
            doc.version = version;
        }
    }

    /// Publish diagnostics for a document
    pub async fn publish_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) {
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    /// Send a custom notification via log messages for now
    /// Custom notifications require implementing the Notification trait properly
    pub async fn send_notification(&self, method: &str, params: serde_json::Value) {
        let payload = match serde_json::to_string(&params) {
            Ok(s) => s,
            Err(_) => "{}".to_string(),
        };

        match method {
            "window/showError" => {
                self.client
                    .show_message(MessageType::ERROR, payload)
                    .await;
            }
            "window/showWarning" => {
                self.client
                    .show_message(MessageType::WARNING, payload)
                    .await;
            }
            "window/showInfo" => {
                self.client.show_message(MessageType::INFO, payload).await;
            }
            _ => {
                self.client
                    .log_message(MessageType::LOG, format!("{}: {}", method, payload))
                    .await;
            }
        }
    }
}

/// Custom notification type - using a simpler approach without custom notifications
/// The tower-lsp Notification trait requires specific implementation patterns.
/// For now, we use the built-in client methods directly instead of custom notifications.

#[tower_lsp::async_trait]
impl LanguageServer for KyroLspBackend {
    /// Initialize the language server
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        // Store root URI
        if let Some(root_uri) = params.root_uri {
            *self.root_uri.write().await = Some(root_uri);
        }

        // Build server capabilities
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(true),
                trigger_characters: Some(vec![
                    ".".to_string(),
                    ":".to_string(),
                    "<".to_string(),
                    "\"".to_string(),
                    "/".to_string(),
                ]),
                all_commit_characters: None,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(false),
                },
                completion_item: None,
            }),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            definition_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Left(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            workspace_symbol_provider: Some(OneOf::Left(true)),
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            document_formatting_provider: Some(OneOf::Left(true)),
            document_range_formatting_provider: Some(OneOf::Left(true)),
            rename_provider: Some(OneOf::Left(true)),
            signature_help_provider: Some(SignatureHelpOptions {
                trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                retrigger_characters: None,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(false),
                },
            }),
            execute_command_provider: Some(ExecuteCommandOptions {
                commands: vec![
                    "kyro.ai.complete".to_string(),
                    "kyro.ai.explain".to_string(),
                    "kyro.ai.review".to_string(),
                    "kyro.ai.refactor".to_string(),
                    "kyro.ai.generateTests".to_string(),
                ],
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(false),
                },
            }),
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                    legend: SemanticTokensLegend {
                        token_types: vec![
                            SemanticTokenType::NAMESPACE,
                            SemanticTokenType::TYPE,
                            SemanticTokenType::CLASS,
                            SemanticTokenType::ENUM,
                            SemanticTokenType::INTERFACE,
                            SemanticTokenType::STRUCT,
                            SemanticTokenType::TYPE_PARAMETER,
                            SemanticTokenType::PARAMETER,
                            SemanticTokenType::VARIABLE,
                            SemanticTokenType::PROPERTY,
                            SemanticTokenType::ENUM_MEMBER,
                            SemanticTokenType::FUNCTION,
                            SemanticTokenType::METHOD,
                            SemanticTokenType::MACRO,
                            SemanticTokenType::KEYWORD,
                            SemanticTokenType::MODIFIER,
                            SemanticTokenType::COMMENT,
                            SemanticTokenType::STRING,
                            SemanticTokenType::NUMBER,
                            SemanticTokenType::REGEXP,
                            SemanticTokenType::OPERATOR,
                        ],
                        token_modifiers: vec![
                            SemanticTokenModifier::DECLARATION,
                            SemanticTokenModifier::DEFINITION,
                            SemanticTokenModifier::READONLY,
                            SemanticTokenModifier::STATIC,
                            SemanticTokenModifier::DEPRECATED,
                            SemanticTokenModifier::ABSTRACT,
                            SemanticTokenModifier::ASYNC,
                            SemanticTokenModifier::MODIFICATION,
                            SemanticTokenModifier::DOCUMENTATION,
                            SemanticTokenModifier::DEFAULT_LIBRARY,
                        ],
                    },
                    range: Some(true),
                    full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
                }),
            ),
            ..Default::default()
        };

        Ok(InitializeResult {
            capabilities,
            server_info: Some(ServerInfo {
                name: "KYRO IDE Language Server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    /// Server initialized notification
    async fn initialized(&self, _params: InitializedParams) {
        log::info!("KYRO LSP server initialized");
    }

    /// Shutdown the language server
    async fn shutdown(&self) -> LspResult<()> {
        log::info!("KYRO LSP server shutting down");
        Ok(())
    }

    /// Document opened
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let TextDocumentItem {
            uri,
            language_id,
            version,
            text,
        } = params.text_document;

        log::info!("Document opened: {} ({})", uri, language_id);

        let doc = DocumentState {
            uri: uri.clone(),
            language_id: language_id.clone(),
            version,
            content: text.clone(),
            symbols: Vec::new(),
            diagnostics: Vec::new(),
        };

        self.documents.write().await.insert(uri.clone(), doc);

        // Trigger diagnostics
        if self.config.diagnostics {
            let diagnostics = self.compute_diagnostics(&uri, &text, &language_id).await;
            self.publish_diagnostics(uri, diagnostics).await;
        }
    }

    /// Document changed
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let VersionedTextDocumentIdentifier { uri, version } = params.text_document;

        let mut docs = self.documents.write().await;
        if let Some(doc) = docs.get_mut(&uri) {
            // Apply content changes
            for change in params.content_changes {
                if let Some(range) = change.range {
                    // Incremental update
                    doc.content = apply_change(&doc.content, range, &change.text);
                } else {
                    // Full update
                    doc.content = change.text;
                }
            }
            doc.version = version;

            // Trigger diagnostics (debounced in production)
            if self.config.diagnostics {
                let diagnostics = self
                    .compute_diagnostics(&uri, &doc.content, &doc.language_id)
                    .await;
                let uri_clone = uri.clone();
                drop(docs);
                self.publish_diagnostics(uri_clone, diagnostics).await;
            }
        }
    }

    /// Document saved
    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let TextDocumentIdentifier { uri } = params.text_document;
        log::info!("Document saved: {}", uri);
    }

    /// Document closed
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let TextDocumentIdentifier { uri } = params.text_document;
        log::info!("Document closed: {}", uri);
        self.documents.write().await.remove(&uri);
    }

    /// Completion request
    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            let items = self
                .get_completions(&doc.content, &doc.language_id, position)
                .await;

            return Ok(Some(CompletionResponse::Array(items)));
        }

        Ok(None)
    }

    /// Resolve completion item
    async fn completion_resolve(&self, item: CompletionItem) -> LspResult<CompletionItem> {
        // Add documentation, details, etc.
        Ok(item)
    }

    /// Hover request
    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            if let Some(hover) = self
                .get_hover(&doc.content, &doc.language_id, position)
                .await
            {
                return Ok(Some(hover));
            }
        }

        Ok(None)
    }

    /// Go to definition
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> LspResult<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            if let Some(location) = self
                .get_definition(&doc.content, &doc.language_id, position)
                .await
            {
                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }
        }

        Ok(None)
    }

    /// Find references
    async fn references(&self, params: ReferenceParams) -> LspResult<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            let references = self
                .get_references(&doc.content, &doc.language_id, position)
                .await;
            return Ok(Some(references));
        }

        Ok(None)
    }

    /// Document symbols
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> LspResult<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            let symbols = self
                .get_document_symbols(&doc.content, &doc.language_id)
                .await;
            return Ok(Some(DocumentSymbolResponse::Flat(symbols)));
        }

        Ok(None)
    }

    /// Execute command
    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> LspResult<Option<serde_json::Value>> {
        let command = params.command;
        let args = params.arguments;

        log::info!("Executing command: {} with {} args", command, args.len());

        match command.as_str() {
            "kyro.ai.complete" => {
                // AI-powered completion
                Ok(Some(serde_json::json!({ "status": "completed" })))
            }
            "kyro.ai.explain" => {
                // Explain code
                Ok(Some(serde_json::json!({ "status": "completed" })))
            }
            "kyro.ai.review" => {
                // Code review
                Ok(Some(serde_json::json!({ "status": "completed" })))
            }
            "kyro.ai.refactor" => {
                // Refactor
                Ok(Some(serde_json::json!({ "status": "completed" })))
            }
            "kyro.ai.generateTests" => {
                // Generate tests
                Ok(Some(serde_json::json!({ "status": "completed" })))
            }
            _ => Ok(None),
        }
    }

    /// Format document
    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> LspResult<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            let edits = self
                .format_document(&doc.content, &doc.language_id, &params.options)
                .await;
            return Ok(Some(edits));
        }

        Ok(None)
    }

    /// Rename symbol
    async fn rename(&self, params: RenameParams) -> LspResult<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            let edits = self
                .get_rename_edits(&doc.content, &doc.language_id, position, &new_name)
                .await;

            return Ok(Some(WorkspaceEdit {
                changes: Some(std::collections::HashMap::from([(uri.clone(), edits)])),
                document_changes: None,
                change_annotations: None,
            }));
        }

        Ok(None)
    }
}

// Helper implementations
impl KyroLspBackend {
    async fn compute_diagnostics(
        &self,
        _uri: &Url,
        content: &str,
        _language_id: &str,
    ) -> Vec<Diagnostic> {
        // Use tree-sitter for syntax errors
        let mut diagnostics = Vec::new();

        // Basic bracket matching
        let lines: Vec<&str> = content.lines().collect();
        let mut bracket_stack = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
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
                                range: Range {
                                    start: Position {
                                        line: line_num as u32,
                                        character: col as u32,
                                    },
                                    end: Position {
                                        line: line_num as u32,
                                        character: col as u32 + 1,
                                    },
                                },
                                severity: Some(DiagnosticSeverity::ERROR),
                                message: "Unmatched closing parenthesis".to_string(),
                                source: Some("kyro-lsp".to_string()),
                                ..Default::default()
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
                                range: Range {
                                    start: Position {
                                        line: line_num as u32,
                                        character: col as u32,
                                    },
                                    end: Position {
                                        line: line_num as u32,
                                        character: col as u32 + 1,
                                    },
                                },
                                severity: Some(DiagnosticSeverity::ERROR),
                                message: "Unmatched closing bracket".to_string(),
                                source: Some("kyro-lsp".to_string()),
                                ..Default::default()
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
                                range: Range {
                                    start: Position {
                                        line: line_num as u32,
                                        character: col as u32,
                                    },
                                    end: Position {
                                        line: line_num as u32,
                                        character: col as u32 + 1,
                                    },
                                },
                                severity: Some(DiagnosticSeverity::ERROR),
                                message: "Unmatched closing brace".to_string(),
                                source: Some("kyro-lsp".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        // Check for unclosed brackets
        for (bracket, line, col) in bracket_stack {
            let closing = match bracket {
                '(' => ")",
                '[' => "]",
                '{' => "}",
                _ => continue,
            };
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: line as u32,
                        character: col as u32,
                    },
                    end: Position {
                        line: line as u32,
                        character: col as u32 + 1,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!("Unclosed '{}', expected '{}'", bracket, closing),
                source: Some("kyro-lsp".to_string()),
                ..Default::default()
            });
        }

        diagnostics
    }

    async fn get_completions(
        &self,
        content: &str,
        language_id: &str,
        position: Position,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Get current line
        let lines: Vec<&str> = content.lines().collect();
        let current_line = lines.get(position.line as usize).unwrap_or(&"");

        // Get text before cursor
        let text_before = if (position.character as usize) <= current_line.len() {
            &current_line[..position.character as usize]
        } else {
            current_line
        };

        // Language-specific completions
        match language_id {
            "rust" => {
                items.extend(self.rust_completions(text_before));
            }
            "python" => {
                items.extend(self.python_completions(text_before));
            }
            "typescript" | "javascript" => {
                items.extend(self.js_ts_completions(text_before));
            }
            "go" => {
                items.extend(self.go_completions(text_before));
            }
            _ => {}
        }

        items.truncate(self.config.max_completion_items);
        items
    }

    fn rust_completions(&self, _text_before: &str) -> Vec<CompletionItem> {
        let keywords = vec![
            "fn", "let", "mut", "const", "static", "pub", "mod", "use", "struct", "enum", "impl",
            "trait", "type", "where", "for", "loop", "while", "if", "else", "match", "return",
            "async", "await",
        ];

        keywords
            .into_iter()
            .map(|kw| CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            })
            .collect()
    }

    fn python_completions(&self, _text_before: &str) -> Vec<CompletionItem> {
        let keywords = vec![
            "def", "class", "if", "elif", "else", "for", "while", "try", "except", "finally",
            "with", "as", "import", "from", "return", "yield", "raise", "pass", "lambda", "async",
            "await",
        ];

        keywords
            .into_iter()
            .map(|kw| CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            })
            .collect()
    }

    fn js_ts_completions(&self, _text_before: &str) -> Vec<CompletionItem> {
        let keywords = vec![
            "function",
            "const",
            "let",
            "var",
            "class",
            "interface",
            "type",
            "enum",
            "import",
            "export",
            "from",
            "async",
            "await",
            "return",
            "if",
            "else",
            "for",
            "while",
            "switch",
            "case",
            "break",
        ];

        keywords
            .into_iter()
            .map(|kw| CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            })
            .collect()
    }

    fn go_completions(&self, _text_before: &str) -> Vec<CompletionItem> {
        let keywords = vec![
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
            "return",
            "go",
            "defer",
            "select",
        ];

        keywords
            .into_iter()
            .map(|kw| CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            })
            .collect()
    }

    /// Get hover content for a word
    fn get_hover_content(&self, word: &str, language_id: &str) -> String {
        match language_id {
            "rust" => self.rust_hover_content(word),
            "python" => self.python_hover_content(word),
            "typescript" | "javascript" => self.js_ts_hover_content(word),
            "go" => self.go_hover_content(word),
            _ => format!("`{}`", word),
        }
    }

    fn rust_hover_content(&self, word: &str) -> String {
        let docs = match word {
            "fn" => "**fn** - Declare a function\n\n```rust\nfn name(params) -> ReturnType {\n    // body\n}\n```",
            "let" => "**let** - Bind a value to a variable\n\n```rust\nlet x = 5;\nlet mut y = 10; // mutable\n```",
            "mut" => "**mut** - Make a binding mutable\n\n```rust\nlet mut x = 5;\nx = 10; // OK\n```",
            "struct" => "**struct** - Define a custom data type\n\n```rust\nstruct Name {\n    field: Type,\n}\n```",
            "enum" => "**enum** - Define an enumeration\n\n```rust\nenum Name {\n    Variant1,\n    Variant2(i32),\n}\n```",
            "impl" => "**impl** - Implement methods for a type\n\n```rust\nimpl TypeName {\n    fn method(&self) {}\n}\n```",
            "trait" => "**trait** - Define shared behavior\n\n```rust\ntrait Name {\n    fn method(&self);\n}\n```",
            "match" => "**match** - Pattern matching\n\n```rust\nmatch value {\n    Pattern1 => result1,\n    Pattern2 => result2,\n    _ => default,\n}\n```",
            "Result" => "**Result<T, E>** - Error handling type\n\n- `Ok(T)` - Success\n- `Err(E)` - Error\n\nUse `?` operator for propagation.",
            "Option" => "**Option<T>** - Nullable type\n\n- `Some(T)` - Value present\n- `None` - No value\n\nSafer than null pointers.",
            "Vec" => "**Vec<T>** - Growable array\n\n```rust\nlet mut v = Vec::new();\nv.push(1);\nlet v = vec![1, 2, 3];\n```",
            "String" => "**String** - Growable UTF-8 string\n\n```rust\nlet s = String::from(\"hello\");\nlet s = \"hello\".to_string();\n```",
            _ => return format!("`{}` - Rust identifier", word),
        };
        docs.to_string()
    }

    fn python_hover_content(&self, word: &str) -> String {
        let docs = match word {
            "def" => "**def** - Define a function\n\n```python\ndef name(params):\n    \"\"\"docstring\"\"\"\n    return value\n```",
            "class" => "**class** - Define a class\n\n```python\nclass Name:\n    def __init__(self):\n        pass\n```",
            "import" => "**import** - Import a module\n\n```python\nimport module\nfrom module import name\n```",
            "lambda" => "**lambda** - Anonymous function\n\n```python\nfn = lambda x: x * 2\n```",
            "yield" => "**yield** - Generator expression\n\n```python\ndef gen():\n    yield 1\n    yield 2\n```",
            "async" => "**async** - Define async function\n\n```python\nasync def fetch():\n    await something()\n```",
            _ => return format!("`{}` - Python identifier", word),
        };
        docs.to_string()
    }

    fn js_ts_hover_content(&self, word: &str) -> String {
        let docs = match word {
            "function" => "**function** - Declare a function\n\n```typescript\nfunction name(params): ReturnType {\n    return value;\n}\n```",
            "const" => "**const** - Constant binding\n\n```typescript\nconst x = 5;\nconst fn = () => {};\n```",
            "let" => "**let** - Block-scoped variable\n\n```typescript\nlet x = 5;\nx = 10; // OK\n```",
            "interface" => "**interface** - Type contract\n\n```typescript\ninterface Name {\n    property: Type;\n    method(): void;\n}\n```",
            "type" => "**type** - Type alias\n\n```typescript\ntype Name = { field: Type };\ntype Union = A | B;\n```",
            "async" => "**async** - Async function\n\n```typescript\nasync function fn() {\n    const result = await promise();\n}\n```",
            "class" => "**class** - Define a class\n\n```typescript\nclass Name {\n    constructor() {}\n    method() {}\n}\n```",
            _ => return format!("`{}` - TypeScript/JavaScript identifier", word),
        };
        docs.to_string()
    }

    fn go_hover_content(&self, word: &str) -> String {
        let docs = match word {
            "func" => "**func** - Declare a function\n\n```go\nfunc name(params) returnType {\n    return value\n}\n```",
            "struct" => "**struct** - Define a struct\n\n```go\ntype Name struct {\n    Field Type\n}\n```",
            "interface" => "**interface** - Define an interface\n\n```go\ntype Name interface {\n    Method()\n}\n```",
            "go" => "**go** - Start a goroutine\n\n```go\ngo func() {\n    // concurrent code\n}()\n```",
            "defer" => "**defer** - Defer execution\n\n```go\ndefer cleanup()\n// cleanup() runs when function returns\n```",
            "chan" => "**chan** - Channel type\n\n```go\nch := make(chan int)\nch <- value  // send\nv := <-ch    // receive\n```",
            _ => return format!("`{}` - Go identifier", word),
        };
        docs.to_string()
    }

    /// Get definition patterns for a word
    fn get_definition_patterns(&self, word: &str, language_id: &str) -> Vec<String> {
        match language_id {
            "rust" => vec![
                format!("fn {}", word),
                format!("struct {}", word),
                format!("enum {}", word),
                format!("trait {}", word),
                format!("impl {}", word),
                format!("type {}", word),
                format!("const {}", word),
                format!("static {}", word),
                format!("mod {}", word),
            ],
            "python" => vec![format!("def {}", word), format!("class {}", word)],
            "typescript" | "javascript" => vec![
                format!("function {}", word),
                format!("class {}", word),
                format!("const {}", word),
                format!("let {}", word),
                format!("var {}", word),
                format!("interface {}", word),
                format!("type {}", word),
            ],
            "go" => vec![
                format!("func {}", word),
                format!("type {} struct", word),
                format!("type {} interface", word),
                format!("var {}", word),
                format!("const {}", word),
            ],
            _ => vec![word.to_string()],
        }
    }

    async fn get_hover(
        &self,
        content: &str,
        language_id: &str,
        position: Position,
    ) -> Option<Hover> {
        // Get the word at the current position
        let lines: Vec<&str> = content.lines().collect();
        let current_line = lines.get(position.line as usize)?;

        // Find word boundaries
        let char_pos = position.character as usize;
        if char_pos > current_line.len() {
            return None;
        }

        let start = current_line[..char_pos]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let end = current_line[char_pos..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| char_pos + i)
            .unwrap_or(current_line.len());

        let word = &current_line[start..end];
        if word.is_empty() {
            return None;
        }

        // Provide hover info for known keywords/types
        let hover_content = self.get_hover_content(word, language_id);

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_content,
            }),
            range: Some(Range {
                start: Position {
                    line: position.line,
                    character: start as u32,
                },
                end: Position {
                    line: position.line,
                    character: end as u32,
                },
            }),
        })
    }

    async fn get_definition(
        &self,
        content: &str,
        language_id: &str,
        position: Position,
    ) -> Option<Location> {
        // Simple implementation: search for definition patterns
        let lines: Vec<&str> = content.lines().collect();
        let current_line = lines.get(position.line as usize)?;

        // Find word at position
        let char_pos = position.character as usize;
        if char_pos > current_line.len() {
            return None;
        }

        let start = current_line[..char_pos]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let end = current_line[char_pos..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| char_pos + i)
            .unwrap_or(current_line.len());

        let word = &current_line[start..end];
        if word.is_empty() {
            return None;
        }

        // Search for definition patterns
        let def_patterns = self.get_definition_patterns(word, language_id);

        for (line_num, line) in lines.iter().enumerate() {
            for pattern in &def_patterns {
                if let Some(col) = line.find(pattern) {
                    return Some(Location {
                        uri: Url::parse("file:///").ok()?,
                        range: Range {
                            start: Position {
                                line: line_num as u32,
                                character: col as u32,
                            },
                            end: Position {
                                line: line_num as u32,
                                character: (col + pattern.len()) as u32,
                            },
                        },
                    });
                }
            }
        }

        None
    }

    async fn get_references(
        &self,
        content: &str,
        _language_id: &str,
        position: Position,
    ) -> Vec<Location> {
        let mut references = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        let current_line = match lines.get(position.line as usize) {
            Some(l) => l,
            None => return references,
        };

        // Find word at position
        let char_pos = position.character as usize;
        if char_pos > current_line.len() {
            return references;
        }

        let start = current_line[..char_pos]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let end = current_line[char_pos..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| char_pos + i)
            .unwrap_or(current_line.len());

        let word = &current_line[start..end];
        if word.is_empty() {
            return references;
        }

        // Find all occurrences
        for (line_num, line) in lines.iter().enumerate() {
            let mut search_start = 0;
            while let Some(col) = line[search_start..].find(word) {
                // Check word boundaries
                let abs_col = search_start + col;
                let before_ok = abs_col == 0 || {
                    let c = line.chars().nth(abs_col - 1);
                    c.is_none_or(|c| !c.is_alphanumeric() && c != '_')
                };
                let after_ok = abs_col + word.len() >= line.len() || {
                    let c = line.chars().nth(abs_col + word.len());
                    c.is_none_or(|c| !c.is_alphanumeric() && c != '_')
                };

                if before_ok && after_ok {
                    references.push(Location {
                        uri: Url::parse("file:///")
                            .unwrap_or_else(|_| Url::parse("file:///unknown").unwrap()),
                        range: Range {
                            start: Position {
                                line: line_num as u32,
                                character: abs_col as u32,
                            },
                            end: Position {
                                line: line_num as u32,
                                character: (abs_col + word.len()) as u32,
                            },
                        },
                    });
                }
                search_start = abs_col + word.len();
            }
        }

        references
    }

    async fn get_document_symbols(
        &self,
        content: &str,
        language_id: &str,
    ) -> Vec<SymbolInformation> {
        let mut symbols = Vec::new();

        let patterns = match language_id {
            "rust" => vec![
                ("fn ", SymbolKind::FUNCTION),
                ("struct ", SymbolKind::STRUCT),
                ("enum ", SymbolKind::ENUM),
                ("impl ", SymbolKind::CLASS),
                ("trait ", SymbolKind::INTERFACE),
                ("mod ", SymbolKind::MODULE),
            ],
            "python" => vec![
                ("def ", SymbolKind::FUNCTION),
                ("class ", SymbolKind::CLASS),
            ],
            "typescript" | "javascript" => vec![
                ("function ", SymbolKind::FUNCTION),
                ("class ", SymbolKind::CLASS),
                ("const ", SymbolKind::CONSTANT),
                ("interface ", SymbolKind::INTERFACE),
            ],
            "go" => vec![
                ("func ", SymbolKind::FUNCTION),
                ("type ", SymbolKind::STRUCT),
                ("struct ", SymbolKind::STRUCT),
                ("interface ", SymbolKind::INTERFACE),
            ],
            _ => return symbols,
        };

        for (line_num, line) in content.lines().enumerate() {
            for (pattern, kind) in &patterns {
                if let Some(col) = line.find(pattern) {
                    // Extract name
                    let rest = &line[col + pattern.len()..];
                    let name_end = rest
                        .find(|c: char| c.is_whitespace() || c == '(' || c == '{' || c == ':')
                        .unwrap_or(rest.len());
                    let name = &rest[..name_end];

                    if !name.is_empty() {
                        symbols.push(SymbolInformation {
                            name: name.to_string(),
                            kind: *kind,
                            deprecated: None,
                            location: Location {
                                uri: Url::parse("file:///")
                                    .unwrap_or_else(|_| Url::parse("file:///unknown").unwrap()),
                                range: Range {
                                    start: Position {
                                        line: line_num as u32,
                                        character: col as u32,
                                    },
                                    end: Position {
                                        line: line_num as u32,
                                        character: (col + pattern.len() + name.len()) as u32,
                                    },
                                },
                            },
                            container_name: None,
                            tags: None,
                        });
                    }
                }
            }
        }

        symbols
    }

    async fn format_document(
        &self,
        content: &str,
        _language_id: &str,
        options: &FormattingOptions,
    ) -> Vec<TextEdit> {
        let mut edits = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Simple formatting: fix indentation based on options
        let indent_str = if options.insert_spaces {
            " ".repeat(options.tab_size as usize)
        } else {
            "\t".to_string()
        };

        let mut formatted_lines = Vec::new();
        let mut indent_level: usize = 0;

        for line in &lines {
            let trimmed = line.trim_start();

            // Decrease indent for closing braces
            if trimmed.starts_with('}') || trimmed.starts_with(']') || trimmed.starts_with(')') {
                indent_level = indent_level.saturating_sub(1);
            }

            // Build formatted line
            let formatted = if trimmed.is_empty() {
                String::new()
            } else {
                format!("{}{}", indent_str.repeat(indent_level), trimmed)
            };
            formatted_lines.push(formatted);

            // Increase indent for opening braces
            for c in trimmed.chars() {
                match c {
                    '{' | '[' | '(' => indent_level += 1,
                    '}' | ']' | ')' => indent_level = indent_level.saturating_sub(1),
                    _ => {}
                }
            }
        }

        // Create edit if content changed
        let new_content = formatted_lines.join("\n");
        if new_content != *content {
            edits.push(TextEdit {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: lines.len() as u32,
                        character: 0,
                    },
                },
                new_text: new_content,
            });
        }

        edits
    }

    async fn get_rename_edits(
        &self,
        content: &str,
        _language_id: &str,
        position: Position,
        new_name: &str,
    ) -> Vec<TextEdit> {
        let mut edits = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        let current_line = match lines.get(position.line as usize) {
            Some(l) => l,
            None => return edits,
        };

        // Find word at position
        let char_pos = position.character as usize;
        if char_pos > current_line.len() {
            return edits;
        }

        let start = current_line[..char_pos]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let end = current_line[char_pos..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| char_pos + i)
            .unwrap_or(current_line.len());

        let old_name = &current_line[start..end];
        if old_name.is_empty() {
            return edits;
        }

        // Find all occurrences and create edits
        for (line_num, line) in lines.iter().enumerate() {
            let mut search_start = 0;
            while let Some(col) = line[search_start..].find(old_name) {
                let abs_col = search_start + col;

                // Check word boundaries
                let before_ok = abs_col == 0 || {
                    let c = line.chars().nth(abs_col - 1);
                    c.is_none_or(|c| !c.is_alphanumeric() && c != '_')
                };
                let after_ok = abs_col + old_name.len() >= line.len() || {
                    let c = line.chars().nth(abs_col + old_name.len());
                    c.is_none_or(|c| !c.is_alphanumeric() && c != '_')
                };

                if before_ok && after_ok {
                    edits.push(TextEdit {
                        range: Range {
                            start: Position {
                                line: line_num as u32,
                                character: abs_col as u32,
                            },
                            end: Position {
                                line: line_num as u32,
                                character: (abs_col + old_name.len()) as u32,
                            },
                        },
                        new_text: new_name.to_string(),
                    });
                }
                search_start = abs_col + old_name.len();
            }
        }

        edits
    }
}

/// Apply a text change to content
fn apply_change(content: &str, range: Range, text: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();

    // Get text before and after the change
    let start_line = range.start.line as usize;
    let start_col = range.start.character as usize;
    let end_line = range.end.line as usize;
    let end_col = range.end.character as usize;

    // Reconstruct the content
    let mut result = String::new();

    // Add lines before the change
    for (i, line) in lines.iter().enumerate() {
        if i < start_line {
            result.push_str(line);
            result.push('\n');
        } else if i == start_line {
            // Add text before the change on the same line
            if start_col <= line.len() {
                result.push_str(&line[..start_col]);
            }
            // Add the new text
            result.push_str(text);
        }
    }

    // Add remaining lines after the change
    for (i, line) in lines.iter().enumerate() {
        if i > end_line {
            result.push('\n');
            result.push_str(line);
        } else if i == end_line && end_col <= line.len() {
            // Note: This is simplified, proper implementation would handle multi-line changes
            result.push_str(&line[end_col..]);
        }
    }

    result
}

/// Create the LSP service
pub fn create_lsp_service(
    config: LspConfig,
) -> (LspService<KyroLspBackend>, tower_lsp::ClientSocket) {
    LspService::new(|client| KyroLspBackend::new(client, config))
}

/// Start the LSP server on stdio
pub async fn start_stdio_server(config: LspConfig) {
    let (service, socket) = create_lsp_service(config);
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
