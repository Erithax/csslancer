use crate::css_language_types::ProvideCssData;

use crate::css_language_types::{PropertyData, AtDirectiveData, PseudoClassData, PseudoElementData};

pub struct CssDataProvider {
    properties: Vec<PropertyData>,
    at_directives: Vec<AtDirectiveData>,
    pseudo_classes: Vec<PseudoClassData>,
    pseudo_elements: Vec<PseudoElementData>,
}


impl ProvideCssData for CssDataProvider {
    fn provide_properties(&self) -> Vec<PropertyData> {
        return self.properties
    }
    fn provide_at_directives(&self) -> Vec<AtDirectiveData> {
        return self.at_directives
    }
    fn provide_pseudo_classes(&self) -> Vec<PseudoClassData> {
        return self.pseudo_classes
    }
    fn provide_pseudo_elements(&self) -> Vec<PseudoElementData> {
        return self.pseudo_elements
    }
}