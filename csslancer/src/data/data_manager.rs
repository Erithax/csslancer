use crate::css_language_types::{AtDirectiveData, EntryStatus, PropertyData, ProvideCssData, PseudoClassData, PseudoElementData};

use super::data_provider::CssDataProvider;
use std::collections::{HashMap, HashSet};

pub struct CssDataManager {
    data_providers: Vec<Box<dyn ProvideCssData + Sync + Send>>,

    property_set: HashMap<String, PropertyData>,
    at_directive_set: HashMap<String, AtDirectiveData>,
    pseudo_class_set: HashMap<String, PseudoClassData>,
    pseudo_element_set: HashMap<String, PseudoElementData>,

    // properties: Vec<&PropertyData>,
    // at_directives: Vec<&AtDirectiveData>,
    // pseudo_classes: Vec<&PseudoClassData>,
    // pseudo_elements: Vec<&PseudoElementData>,
}

impl CssDataManager {
    pub fn new(use_default_data_provider: bool, custom_data_providers: Option<Vec<Box<dyn ProvideCssData + Sync>>>) -> Self {
        let mut data_providers = Vec::new();
        if use_default_data_provider {
            data_providers.push(CssDataProvider::new(CssData));
        }
        if let Some(custom_data_providers) = custom_data_providers {
            data_providers.append(&mut custom_data_providers);
        }

        let mut res = Self::default();
        res.data_providers = data_providers;
        res.collect_data();
        return res
    }

    fn collect_data(&mut self) {
        for data_provider in self.data_providers {
            data_provider.provide_properties().into_iter().for_each(|i| {self.property_set.insert(i.name, i);});
            data_provider.provide_at_directives().into_iter().for_each(|i| {self.at_directive_set.insert(i.name, i);});
            data_provider.provide_pseudo_classes().into_iter().for_each(|i| {self.pseudo_class_set.insert(i.name, i);});
            data_provider.provide_pseudo_elements().into_iter().for_each(|i| {self.pseudo_element_set.insert(i.name, i);});
        }
        // self.properties = self.property_set.iter().collect();
        // self.at_directives = self.at_directive_set.iter().collect();
        // self.pseudo_classes = self.pseudo_class_set.iter().collect();
        // self.pseudo_elements = self.pseudo_element_set.iter().collect();
    }

    pub fn get_property(&self, name: &str) -> Option<&PropertyData> {return self.property_set.get(name)}
    pub fn get_at_directive(&self, name: &str) -> Option<&AtDirectiveData> {return self.at_directive_set.get(name)}
    pub fn get_pseudo_class(&self, name: &str) -> Option<&PseudoClassData> {return self.pseudo_class_set.get(name)}
    pub fn get_pseudo_element(&self, name: &str) -> Option<&PseudoElementData> {return self.pseudo_element_set.get(name)}

    pub fn is_known_property(&self, name: &str) -> bool {
        let name = name.to_lowercase();
        return self.property_set.contains_key(name.as_str());
    }

    pub fn is_standard_property(&self, name: &str) -> bool {
        return if let Some(prop) = self.property_set.get(name.to_lowercase().as_str()) {
            prop.status == Some(EntryStatus::Standard)
        } else {
            false
        };
    }
    
}

impl Default for CssDataManager {
    fn default() -> Self {
        Self {
            property_set: HashMap::new(),
            at_directive_set: HashMap::new(),
            pseudo_class_set: HashMap::new(),
            pseudo_element_set: HashMap::new(),
            ..Default::default()
        }
    }
}