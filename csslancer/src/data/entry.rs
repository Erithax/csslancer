

use regex::Regex;
use tracing::trace;

use crate::css_language_types::{ AtDirectiveData, Content, EntryStatus, HoverSettings, MarkedString, MarkupContent, MarkupKind, PropertyData, PseudoClassData, PseudoElementData, Reference, ValueData };

// struct Browsers {
//     E?: string;
//     FF?: string;
//     IE?: string;
//     O?: string;
//     C?: string;
//     S?: string;
//     count: number;
//     all: boolean;
//     onCodeComplete: boolean;
// }

pub const BROWSER_NAMES: [(&str, &str); 6] = [
    ("E", "Edge"),
    ("FF", "Firefox"),
    ("S", "Safari"),
    ("C", "Chrome"),
    ("IE", "IE"),
    ("O", "Opera"),
];

const fn get_entry_status(status: EntryStatus) -> &'static str {
    match status {
        EntryStatus::Experimental => "Property is experimental. Be cautious when using it.️\n\n'",
        EntryStatus::NonStandard => "Property is nonstandard. Avoid using it.\n\n",
        EntryStatus::Obsolete => "Property is obsolete. Avoid using it.\n\n",
        EntryStatus::Standard => "",
    }
}

pub fn get_entry_description(entry: IEntry2, does_support_markdown: bool, settings: &Option<HoverSettings>) -> Option<MarkupContent> {

    let result: MarkupContent = if does_support_markdown {
        MarkupContent {
            kind: MarkupKind::Markdown,
            value: get_entry_markdown_description(entry, settings)
        }
    } else {
        MarkupContent {
            kind: MarkupKind::PlainText,
            value: get_entry_string_description(entry, settings)
        }
    };

    if result.value == "" {
        return None;
    }

    return Some(result)
}

pub fn text_to_marked_string(mut text: String) -> MarkedString {
    markify_string(&mut text);
    return MarkedString::String(text);
}

pub fn text_to_marked_string_inner(mut text: String) -> String {
    markify_string(&mut text);
    return text
}

// escape markdown syntax tokens
pub fn markify_string(text: &mut String) {
    for ch in "\\[]{}()`*#+-.!".chars() {
        *text = text.replace(&ch.to_string(), &("\\".to_owned() + &ch.to_string()));
    }
    *text = text.replace("<", "&lt;");
    *text = text.replace(">", "&gt;");
    //text = text.replace("[\\`*_{}[\\]()#+\\-.!]", "\\$&"); // escape markdown syntax tokens: http://daringfireball.net/projects/markdown/syntax#backslash
    //return text.replace("<", "&lt;").replace(">", "&gt;");
}

fn get_entry_string_description(entry: IEntry2, settings: &Option<HoverSettings>) -> String {
    let Some(desc) = entry.description() else {
        return "".to_owned();
    };
    if desc.value() == "" {
        return "".to_owned();
    }

    let Content::String(_) = desc else {
        return desc.value().to_owned();
    };

    let mut result = String::new();

    if let Some(settings) = settings {
        if settings.documentation != false {
            if let Some(status) = entry.status() {
                result += get_entry_status(*status);
            }
            result += &desc.value();
    
            if let Some(browser_label) = get_browser_label(&entry.browsers().as_ref().unwrap_or(&Vec::new())) {
                result += "\n(";
                result += &browser_label;
                result += ")";
            }
            if let Some(syntax) = entry.syntax() {
                result += &format!("\n\nSyntax: {syntax}");
            }
        }

        if let Some(refs) = entry.references() {
            if refs.len() > 0 && settings.references != false {
                if result.len() > 0 {
                    result += "\n\n";
                }
                result += &refs.into_iter().map(|r| {
                    return r.name.to_owned() + ": " + &r.url
                }).collect::<Vec<String>>().join(" | ");
            }
        }
    }

    return result;
}

fn get_entry_markdown_description(entry: IEntry2, settings: &Option<HoverSettings>) -> String {
    let Some(desc) = entry.description() else {
        return "".to_owned();
    };
    if desc.value() == "" {
        return "".to_owned();
    }

    let mut result = String::new();
    if let Some(settings) = settings {
        if settings.documentation != false {
            if let Some(status) = entry.status() {
                result += get_entry_status(*status);
            }

            match desc {
                Content::String(s) => result += &text_to_marked_string_inner(s.to_string()),
                Content::Markup(mc) => {
                    match mc.kind {
                        MarkupKind::Markdown => result += &mc.value,
                        MarkupKind::PlainText => result += &text_to_marked_string_inner(mc.value.clone()),
                    }
                }
            };
        
            if let Some(browser_label) = get_browser_label(&entry.browsers().as_ref().unwrap_or(&Vec::new())) {
                result += "\n\n(";
                result += &text_to_marked_string_inner(browser_label);
                result += ")";
            }
            if let Some(syntax) = entry.syntax() {
                result += "\n\nSyntax: ";
                result += &text_to_marked_string_inner(syntax.to_owned());
            }
        }

        if let Some(refs) = entry.references() {
            if refs.len() > 0 && settings.references != false {

                if result.len() > 0 {
                    result += "\n\n";
                }
                result += &refs.into_iter().map(|r| {
                    return "[".to_owned() + &r.name + "](" + &r.url + ")";
                }).collect::<Vec<String>>().join(" | ");
            }
        }

    }


    return result;
}

/**
 * Input is like `["E12","FF49","C47","IE","O"]`
* Output is like `Edge 12, Firefox 49, Chrome 47, IE, Opera`
*/
pub fn get_browser_label(browsers: &Vec<String>) -> Option<String> {
    if browsers.len() == 0 {
        return None;
    }

    return Some(browsers
        .into_iter().map(|b| {
            let mut result = "".to_owned();
            let reg = Regex::new(r"(?<name>[A-Z]+)(?<version>\d+)?").unwrap();
            let mut matches = reg.captures_iter(&b);

            let first_mat = matches.next();
            let name = first_mat.as_ref().map(|f| f["name"].to_owned());
            let version = first_mat.map(|f| f["version"].to_owned());

            if let Some(name) = name {
                trace!(name = name);

                if let Some(browso) = BROWSER_NAMES.iter().find(|b| b.0 == name) {
                    result += browso.1;
                }
            }
            if let Some(version) = version {
                result += " ";
                result += &version;
            }
            return result;
        }).collect::<Vec<String>>().join(", "));
}



pub enum IEntry2<'a> {
    Prop(&'a PropertyData),
    AtDir(&'a AtDirectiveData),
    PseuClass(&'a PseudoClassData),
    PseuEle(&'a PseudoElementData),
    Value(&'a ValueData),
}

impl IEntry2<'_> {
    pub fn description(&self) -> &Option<Content> {
        match self {
            Self::Prop(PropertyData {description, ..}) |
            Self::AtDir(AtDirectiveData {description, ..}) |
            Self::PseuClass(PseudoClassData {description, ..}) |
            Self::PseuEle(PseudoElementData {description, ..}) |
            Self::Value(ValueData {description, ..}) => {
                description
            }
        }
    }

    pub fn status(&self) -> &Option<EntryStatus> {
        match self {
            Self::Prop(PropertyData {status, ..}) |
            Self::AtDir(AtDirectiveData {status, ..}) |
            Self::PseuClass(PseudoClassData {status, ..}) |
            Self::PseuEle(PseudoElementData {status, ..}) |
            Self::Value(ValueData {status, ..}) => {
                status
            }
        }
    }

    pub fn browsers(&self) -> &Option<Vec<String>> {
        match self {
            Self::Prop(PropertyData {browsers, ..}) |
            Self::AtDir(AtDirectiveData {browsers, ..}) |
            Self::PseuClass(PseudoClassData {browsers, ..}) |
            Self::PseuEle(PseudoElementData {browsers, ..}) |
            Self::Value(ValueData {browsers, ..}) => {
                browsers
            }
        }
    }

    pub fn syntax(&self) -> Option<&String> {
        match self {
            Self::Prop(PropertyData {syntax, ..}) => {syntax.as_ref()}
            _ => {None}
        }
    }

    pub fn references(&self) -> &Option<Vec<Reference>> {
        match self {
            Self::Prop(PropertyData {references, ..}) |
            Self::AtDir(AtDirectiveData {references, ..}) |
            Self::PseuClass(PseudoClassData {references, ..}) |
            Self::PseuEle(PseudoElementData {references, ..}) |
            Self::Value(ValueData {references, ..}) => {
                references
            }
        }
    }

}




// /**
//  * Todo@Pine: Drop these two types and use IEntry2
// */
// pub interface IEntry {
//     name: string;
//     description?: string | MarkupContent;
//     browsers?: string[];
//     restrictions?: string[];
//     status?: EntryStatus;
//     syntax?: string;
//     values?: IValue[];
// }

// pub interface IValue {
//     name: string;
//     description?: string | MarkupContent;
//     browsers?: string[];
// }