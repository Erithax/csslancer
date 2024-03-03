use crate::css_language_types::*;

use std::collections::HashMap;

impl Default for LanguageSettings {
    fn default() -> Self {
        return LanguageSettings {
            validate: true,
            lint: HashMap::new(),
            completion: CompletionSettings {
                trigger_property_value_completion: true,
                complete_property_with_semicolon: true,
            },
            hover: HoverSettings {
                documentation: true,
                references: true,
            },
            alias: HashMap::new(),
        };
    }
}
