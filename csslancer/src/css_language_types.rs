use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
pub use lsp_types::*;

type LintSettings = HashMap<String, String>;

pub struct CompletionSettings {
    pub trigger_property_value_completion: bool,
    pub complete_property_with_semicolon: bool,
}

type AliasSettings = HashMap<String, String>;

pub struct HoverSettings {
    pub documentation: bool,
    pub references: bool,
}

pub struct LanguageSettings {
    pub validate: bool,
    pub lint: LintSettings,
    pub completion: CompletionSettings,
    pub hover: HoverSettings,
    pub alias: AliasSettings,
}

//

pub struct PropertyCompletionContext {
    property_name: String,
    range: Range,
}

pub struct PropertyValueCompletionContext {
    property_name: String,
    property_value: String,
    range: Range,
}

pub struct URILiteralCompletionContext {
    uri_value: String,
    position: Position,
    range: Range,
}

pub struct ImportPathCompletionContext {
    path_value: String,
    position: Position,
    range: Range,
}

pub struct MixinReferenceCompletionContext {
    mixin_name: String,
    range: Range,
}

pub trait CompletionParticipant {
    fn on_css_property(context: PropertyCompletionContext)
    where
        Self: Sized;
    fn on_css_property_value(context: PropertyValueCompletionContext)
    where
        Self: Sized;
    fn on_css_URI_literal_value(context: URILiteralCompletionContext)
    where
        Self: Sized;
    fn on_css_import_path(context: ImportPathCompletionContext)
    where
        Self: Sized;
    fn on_css_mixin_reference(context: MixinReferenceCompletionContext)
    where
        Self: Sized;
}

pub trait DocumentContext {
    fn resolve_reference(&self, reference: String, base_url: String) -> Option<String>;
}

//

type DocumentationFormatCapability = Option<&'static [MarkupKind]>;
type ContentFormatCapability = Option<&'static [MarkupKind]>;

pub enum CompletionItemCapability {
    None,
    Some(DocumentationFormatCapability),
}

pub enum CompletionCapability {
    None,
    Some(CompletionItemCapability),
}

pub struct ClientCapabilities {
    // TODO, see https://github.com/microsoft/vscode-css-languageservice/blob/main/src/cssLanguageTypes.ts
    pub text_document: Option<CompletionCapability>,
    pub hover: ContentFormatCapability,
}

const LATEST: ClientCapabilities = ClientCapabilities {
    text_document: Some(CompletionCapability::Some(CompletionItemCapability::Some(
        DocumentationFormatCapability::Some(&[MarkupKind::Markdown, MarkupKind::PlainText]),
    ))),
    hover: ContentFormatCapability::Some(&[MarkupKind::Markdown, MarkupKind::PlainText]),
};

pub struct LanguageServiceOptions {
    /*
     * Unless set to false, the default CSS data provider will be used
     * along with the providers from customDataProviders.
     * Defaults to true.
     */
    use_default_data_provider: bool,

    /*
     * Provide data that could enhance the service's understanding of
     * CSS property / at-rule / pseudo-class / pseudo-element
     */
    custom_data_providers: Vec<Box<dyn ICSSDataProvider>>,

    /*
     * Abstract file system access away from the service.
     * Used for dynamic link resolving, path completion, etc.
     */
    file_system_provider: Box<dyn FileSystemProvider>,

    /*
     * Describes the LSP capabilities the client supports.
     */
    client_capabilities: ClientCapabilities,
}

pub enum EntryStatus {
    Standard,
    Experimental,
    NonStandard,
    Obsolete,
}

//

pub struct Reference {
    name: String,
    url: String,
}

pub enum Content {
    String(String),
    Markup(MarkupContent),
}

pub struct PropertyData {
    name: String,
    description: Content,
    browsers: Vec<String>,
    restrictions: Vec<String>,
    status: EntryStatus,
    syntax: String,
    values: Vec<ValueData>,
    references: Vec<Reference>,
    relevance: i64,
    at_rule: String,
}

pub struct AtDirectiveData {
    name: String,
    description: Content,
    browsers: Vec<String>,
    status: EntryStatus,
    references: Vec<Reference>,
}

type PseudoClassData = AtDirectiveData;
type PseudoElementData = AtDirectiveData;
type ValueData = AtDirectiveData;

pub enum CSSDataVersion {
    One,
    OneOne,
}
impl CSSDataVersion {
    pub fn get_num(&self) -> f32 {
        match self {
            Self::One => 1.0,
            Self::OneOne => 1.1,
        }
    }
}

pub struct CSSDataV1 {
    version: CSSDataVersion,
    properties: Vec<PropertyData>,
    at_directives: Vec<AtDirectiveData>,
    pseudo_classes: Vec<PseudoClassData>,
    pseudo_elements: Vec<PseudoElementData>,
}

pub trait ICSSDataProvider {
    fn provide_properties() -> Vec<PropertyData>
    where
        Self: Sized;
    fn provide_at_directive() -> Vec<AtDirectiveData>
    where
        Self: Sized;
    fn provide_pseudo_classes() -> Vec<PseudoClassData>
    where
        Self: Sized;
    fn provide_pseudo_elements() -> Vec<PseudoElementData>
    where
        Self: Sized;
}

//

pub enum FileType {
    Unknown,
    File,
    Directory,
    SymbolicLink,
}

pub struct FileStat {
    typ: FileType,
    ctime: u64, // The creation timestamp in milliseconds elapsed since January 1, 1970 00:00:00 UTC.
    mtime: u64,
    size: u64, // in bytes
}

#[async_trait]
pub trait FileSystemProvider {
    async fn stat(uri: PathBuf) -> FileStat
    where
        Self: Sized;
    async fn read_directory(uri: PathBuf) -> Vec<(String, FileType)>
    where
        Self: Sized;
}

pub enum BraceStyle {
    Collapse,
    Expand,
}

pub struct CSSFormatConfiguration {
    // indentation size. Default: 4
    tab_size: u32,
    // Whether to use spaces or tabs
    insert_spaces: bool,
    // end with a newline: Default: false
    insert_final_new_line: bool,
    // separate selectors with newline (e.g. "a,\nbr" or "a, br"): Default: true
    newline_between_selectors: bool,
    // add a new line after every css rule: Default: true
    newline_between_rules: bool,
    // ensure space around selector separators:  '>', '+', '~' (e.g. "a>b" -> "a > b"): Default: false
    space_around_selector_seperator: bool,
    // put braces on the same line as rules (`collapse`), or put braces on own line, Allman / ANSI style (`expand`). Default `collapse`
    brace_style: BraceStyle,
    // whether existing line breaks before elements should be preserved. Default: true
    preserve_new_lines: bool,
    // maximum number of line breaks to be preserved in one chunk. Default: unlimited
    max_preserve_new_lines: u32,
    // maximum amount of characters per line (0/undefined = disabled). Default: disabled
    wrap_line_length: u32,
    // add indenting whitespace to empty lines. Default: false
    indent_empty_lines: bool,
}
