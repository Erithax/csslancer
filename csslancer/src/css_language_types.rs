#![allow(dead_code)] // TODO: remove

use std::collections::HashMap;
use std::hash::Hash;
use std::path::PathBuf;

use async_trait::async_trait;
pub use lsp_types::*;
use serde::de;

type LintSettings = HashMap<String, String>;

pub struct CompletionSettings {
    pub trigger_property_value_completion: bool,
    pub complete_property_with_semicolon: bool,
}

type AliasSettings = HashMap<String, String>;

#[derive(Debug, Default)]
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
    fn on_css_uri_literal_value(context: URILiteralCompletionContext)
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
    custom_data_providers: Vec<Box<dyn ProvideCssData>>,

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

#[derive(Debug, PartialEq, Eq, Copy, Clone, serde::Deserialize, serde::Serialize)]
pub enum EntryStatus {

    Standard,
    #[serde(alias = "experimental")] 
    Experimental,
    #[serde(alias = "nonstandard")] 
    NonStandard,
    #[serde(alias = "obsolete")] 
    Obsolete,
}

//

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Reference {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum Content {
    String(String),
    Markup(MarkupContent),
}

struct ContentVisitor;

impl<'de> serde::de::Visitor<'de> for ContentVisitor {
    type Value = Content;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where E: de::Error,
    {
        Ok(Content::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where E: de::Error,
    {
        Ok(Content::String(value))
    }

}
impl<'de> serde::Deserialize<'de> for Content {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de> {
        deserializer.deserialize_string(ContentVisitor)
    }
}


impl Content {
    pub fn value(&self) -> &str {
        match self {
            Self::String(s) => s,
            Self::Markup(mc) => &mc.value,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PropertyData {
    pub name: String,
    pub description: Option<Content>,
    pub browsers: Option<Vec<String>>,
    pub restrictions: Option<Vec<String>>,
    pub status: Option<EntryStatus>,
    pub syntax: Option<String>,
    #[serde(default)] 
    pub values: Vec<ValueData>,
    pub references: Option<Vec<Reference>>,
    pub relevance: i64,
    pub at_rule: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PropertyDataSource {
    pub name: String,
    pub description: Option<String>,
    pub browsers: Option<Vec<String>>,
    pub restrictions: Option<Vec<String>>,
    pub status: Option<EntryStatus>,
    pub syntax: Option<String>,
    #[serde(default)] 
    pub values: Vec<AtDirectiveDataSource>,
    pub references: Option<Vec<Reference>>,
    pub relevance: i64,
    pub at_rule: Option<String>,
}

impl From<PropertyDataSource> for PropertyData {
    fn from(value: PropertyDataSource) -> Self {
        PropertyData {
            name: value.name,
            description: value.description.map(Content::String),
            browsers: value.browsers,
            restrictions: value.restrictions,
            status: value.status,
            syntax: value.syntax,
            values: value.values.into_iter().map(|v| v.into()).collect(),
            references: value.references,
            relevance: value.relevance,
            at_rule: value.at_rule,
        }
    }
}

impl Hash for PropertyData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.name.clone().into_bytes());
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AtDirectiveData {
    pub name: String,
    pub description: Option<Content>,
    pub browsers: Option<Vec<String>>,
    pub status: Option<EntryStatus>,
    pub references: Option<Vec<Reference>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AtDirectiveDataSource {
    pub name: String,
    pub description: Option<String>,
    pub browsers: Option<Vec<String>>,
    pub status: Option<EntryStatus>,
    pub references: Option<Vec<Reference>>,
}

impl From<AtDirectiveDataSource> for AtDirectiveData {
    fn from(value: AtDirectiveDataSource) -> Self {
        AtDirectiveData {
            name: value.name,
            description: value.description.map(Content::String),
            browsers: value.browsers,
            status: value.status,
            references: value.references,
        }
    }
}

impl Hash for AtDirectiveData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(&self.name.clone().into_bytes());
    }
}

pub type PseudoClassData = AtDirectiveData;
pub type PseudoElementData = AtDirectiveData;
pub type ValueData = AtDirectiveData;

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum CssDataVersion {
    One,
    OneOne,
}
impl CssDataVersion {
    pub fn get_num(&self) -> f32 {
        match self {
            Self::One => 1.0,
            Self::OneOne => 1.1,
        }
    }
}

impl TryFrom<f64> for CssDataVersion {
    type Error = String;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if (value - 1.1).abs() < 0.0000001 {return Ok(Self::OneOne)}
        if (value - 1.0).abs() < 0.0000001 {return Ok(Self::One)}
        Err(format!("invalid version float `{}`", value))
    }
}
impl From<CssDataVersion> for f64 {
    fn from(value: CssDataVersion) -> Self {
        match value {
            CssDataVersion::One => 1.0,
            CssDataVersion::OneOne => 1.1
        }
    }
}
// struct CssDataVersionVisitor;

// impl<'de> serde::de::Visitor<'de> for CssDataVersionVisitor {
//     type Value = CssDataVersion;

//     fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         formatter.write_str("an integer between -2^31 and 2^31")
//     }

//     fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
//         where E: de::Error,
//     {
//         if let Ok(val) = CssDataVersion::try_from(value) {
//             Ok(val)
//         } else {
//             Err(E::custom(format!("version number in css data could not be deserialized to known version: {}", value)))
//         }
//     }
// }
// impl<'de> serde::Deserialize<'de> for CssDataVersion {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//         where
//             D: de::Deserializer<'de> {
//         deserializer.deserialize_f64(CssDataVersionVisitor)
//     }
// }
// impl serde::Serialize for CssDataVersion {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//         where
//             S: serde::Serializer {
//         println!("serializing yeya");
//         serializer.serialize_f64(f64::from(*self))
//     }
// }

#[derive(serde::Deserialize, serde::Serialize)]
pub struct CssDataV1 {
    pub version: CssDataVersion,
    pub properties: Vec<PropertyData>,
    #[serde(alias = "atDirectives")]
    pub at_directives: Vec<AtDirectiveData>,
    #[serde(alias = "pseudoClasses")]
    pub pseudo_classes: Vec<PseudoClassData>,
    #[serde(alias = "pseudoElements")]
    pub pseudo_elements: Vec<PseudoElementData>,
}


#[derive(serde::Deserialize, serde::Serialize)]
pub struct CssDataV1Source {
    pub version: f64,
    pub properties: Vec<PropertyDataSource>,
    #[serde(alias = "atDirectives")]
    pub at_directives: Vec<AtDirectiveDataSource>,
    #[serde(alias = "pseudoClasses")]
    pub pseudo_classes: Vec<AtDirectiveDataSource>,
    #[serde(alias = "pseudoElements")]
    pub pseudo_elements: Vec<AtDirectiveDataSource>,
}

impl From<CssDataV1Source> for CssDataV1 {
    fn from(value: CssDataV1Source) -> Self {
        Self { 
            version: value.version.try_into().unwrap(), 
            properties: value.properties.into_iter().map(|x| x.into()).collect(), 
            at_directives: value.at_directives.into_iter().map(|x| x.into()).collect(), 
            pseudo_classes: value.pseudo_classes.into_iter().map(|x| x.into()).collect(), 
            pseudo_elements: value.pseudo_elements.into_iter().map(|x| x.into()).collect(), 
        }
    }
}

pub trait ProvideCssData {
    fn provide_properties(&mut self) -> Vec<PropertyData>;
    fn provide_at_directives(&mut self) -> Vec<AtDirectiveData>;
    fn provide_pseudo_classes(&mut self) -> Vec<PseudoClassData>;
    fn provide_pseudo_elements(&mut self) -> Vec<PseudoElementData>;
}


impl ProvideCssData for CssDataV1 {
    fn provide_properties(&mut self) -> Vec<PropertyData> {
        std::mem::take(&mut self.properties)
    }
    fn provide_at_directives(&mut self) -> Vec<AtDirectiveData> {
        std::mem::take(&mut self.at_directives)
    }
    fn provide_pseudo_classes(&mut self) -> Vec<PseudoClassData> {
        std::mem::take(&mut self.pseudo_classes)
    }
    fn provide_pseudo_elements(&mut self) -> Vec<PseudoElementData> {
        std::mem::take(&mut self.pseudo_elements)
    }
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
