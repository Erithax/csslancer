
use ego_tree::{NodeId, NodeRef, Tree};
use lsp_types::{LanguageString, MarkedString};
use regex::{Regex, RegexBuilder};

use crate::{ext::TreeAttach, parser::{css_node_types::{BodyDeclaration, BodyDeclarationType, CssNodeType}, css_nodes::{CssNode, CssNodeTree}, css_scanner::Scanner}};
use crate::data::data_manager::CssDataManager;

use super::hover::FlagOpts;


#[derive(Debug, Clone)]
struct Attribute {
    name: String,
    value: String,
}

#[derive(Debug, Clone, Default)]
struct Element {
    attributes: Vec<Attribute>,
}

type ElementTree = ego_tree::Tree<Element>;

impl Element {
    pub fn get_attribute_mut(&mut self, name: &str) -> Option<&mut Attribute> {
        for attribute in self.attributes.iter_mut() {
            if attribute.name == name {
                return Some(attribute)
            }
        }
        return None
    }

    // pub fn get_value(&self, name: &str) -> Option<String> {
    //     for attribute in self.attributes.iter() {
    //         if attribute.name == name {
    //             return Some(attribute.value.clone())
    //         }
    //     }
    //     return None
    // }

    pub fn get_value_ref(&self, name: &str) -> Option<&str> {
        for attribute in self.attributes.iter() {
            if attribute.name == name {
                return Some(&attribute.value)
            }
        }
        return None
    }

    pub fn append(&mut self, text: &str) {
        if let Some(last) = self.attributes.last_mut() {
            last.value += text;
        }
    }

    pub fn prepend(&mut self, text: String) {
        if let Some(first) = self.attributes.first_mut() {
            first.value = text + &first.value;
        }
    }

    pub fn add_attr(&mut self, attr: Attribute) {
        if let Some(attr) = self.get_attribute_mut(&attr.name) {
            let b = attr.value.clone();
            attr.value += " ";
            attr.value += &b;
        } else {
            self.attributes.push(attr);
        }
    }

    pub fn add_attrib(&mut self, name: &str, value: String) {
        self.add_attr(Attribute {
            name: name.to_owned(),
            value,
        });
    }
}

struct MarkedStringPrinter {
    result: Vec<String>,
    quote: String,
}

impl MarkedStringPrinter {

    pub fn new(quote: String) -> Self {
        return Self {
            result: Vec::new(),
            quote,
        }
    }

    pub fn print(&mut self, tree: ElementTree, ele_id: NodeId, flag_opts: Option<FlagOpts>) -> Vec<MarkedString> {
        let element = tree.get(ele_id).unwrap();
        if element.id() == tree.root().id() {
            if element.has_children() {
                self.do_print(element.children(), 0);
            }
        } else {
            self.do_print([element], 0);
        }
        let value = match flag_opts {
            Some(fo) => format!("{}\n … ", fo.text) + &self.result.iter().fold(String::new(), |acc, nex| acc + "\n" + nex),
            None => self.result.iter().fold(String::new(), |acc, nex| acc + "\n" + nex)
        };
        return vec![MarkedString::LanguageString(LanguageString {
            language: "html".to_string(),
            value,
        })]
    }

    fn do_print<'a>(&mut self, nodes: impl IntoIterator<Item = NodeRef<'a, Element>>, ident: usize) {
        for node in nodes {
            self.do_print_ele(node, ident);
            if node.has_children() {
                self.do_print(node.children(), ident + 1);
            }
        }
    }

    fn write_line(&mut self, level: usize, content: &str) {
        let ident = "  ".repeat(level + 1);
        self.result.push(ident + content);
    }

    fn do_print_ele(&mut self, node: NodeRef<Element>, ident: usize) {
        let element = node.value();
        let name = element.get_value_ref("name");

        // special case: a simple label
        

        let mut content = "<".to_string();

        // element name
        if let Some(n) = name {
            content += &n;
        } else {
            content += "element";
        }


        // attributes
        for attr in element.attributes.iter() {
            if attr.name != "name" {
                content += " ";
                content += &attr.name;
                content += "=";
                Quotes::ensure(&mut content, &attr.value, &self.quote);
            }
        }

        content += ">";
        self.write_line(ident, &content);

    }

}

struct Quotes;
impl Quotes {
    pub fn ensure(content: &mut String, value: &str, which: &str) {
        content.push_str(which);
        content.push_str(&Self::remove(value));
        content.push_str(which);
    }

    pub fn remove(value: &str) -> &str {
        let reg = Regex::new("^['\"](.*)['\"]$").unwrap();
        let mut mat = reg.find_iter(&value);
        if let Some(_) = mat.next() {
            return mat.next().expect("Only one match, vscode-css-languageservice is coded like this.").as_str();
        }
        return value;
    }
}

#[derive(Default)]
struct Specificity {
    // count of identifiers (e.g. `#app`)
    pub id: usize,
    // count of attributes (`[type="number"]`), classes (`.container-fluid`), and pseudo-classes (`:hover`)
    pub attr: usize,
    // count of tag names (`div`), and pseudo-elements (`::before`)
    pub tag: usize,
}

impl std::ops::Add for Specificity {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self::Output {
        self.id += rhs.id;
        self.attr += rhs.attr;
        self.tag += rhs.tag;
        return self
    }
}

impl std::ops::AddAssign for Specificity {
    fn add_assign(&mut self, rhs: Self) {
        self.id += rhs.id;
        self.attr += rhs.attr;
        self.tag += rhs.tag;
    }
}

// clones node and all ancestors
fn clone_to_root(ele_tree: &Tree<Element>, node_id: NodeId) -> Tree<Element> {
    let mut res = Tree::new(ele_tree.root().value().clone());
    let root_id = res.root().id();

    let mut curr_node = res.orphan(ele_tree.get(node_id).unwrap().value().clone()).id();

    while let Some(par) = res.get(curr_node).unwrap().parent() {
        if par.id() == root_id {break}
        let par_clone = par.value().clone();
        curr_node = par.id();
        let mut next = res.orphan(par_clone);
        next.append_id(curr_node);
    }

    res.root_mut().append_id(curr_node);
    return res
}

// returns detached stem of cloned nodes
fn clone_to_root_in_tree(ele_tree: &mut Tree<Element>, node_id: NodeId) -> NodeId {
    let root_id = ele_tree.root().id();

    let mut curr_orig_node_id = node_id;
    let mut curr_node_id = ele_tree.orphan(ele_tree.get(node_id).unwrap().value().clone()).id();

    while let Some(par) = ele_tree.get(curr_orig_node_id).unwrap().parent() {
        if par.id() == root_id {break}
        curr_orig_node_id = par.id();
        let mut next: ego_tree::NodeMut<'_, Element> = ele_tree.orphan(par.value().clone());
        next.append_id(curr_node_id);
        curr_node_id = next.id();
    }

    return curr_node_id
}

fn to_element(tree: &CssNodeTree, node_id: NodeId, ele_tree: Option<&Tree<Element>>, parent_element: Option<NodeId>) -> Tree<Element> {
    assert!(!ele_tree.is_none() || parent_element.is_none());
    let node = tree.get(node_id).unwrap();
    
    let mut res_tree = Tree::new(Element {
        attributes: Vec::new(),
    });
    let mut res = res_tree.root().id();

    macro_rules! res_val_mut {
        () => {
            res_tree.get_mut(res).unwrap().value()
        };
    }
    
    for child in node.children() {
        use CssNodeType::*;
        match &child.value().node_type {
            SelectorCombinator => {
                if let (Some(ele_tree), Some(parent_element)) = (&ele_tree, parent_element) {
                    let segments: Vec<&str> = tree.get_text(node.id()).split("&").collect();
                    assert!(segments.len() != 1);
                    // if segments.len() == 1 {
                    //     // should not happen
                    //     res_tree.get_mut(res).unwrap().value().add_attr(Attribute {name: "name".to_owned(), value: segments.first().unwrap().to_owned().to_owned()});
                    //     break //todo!()
                    // }
                    res_tree = clone_to_root(ele_tree, parent_element);
                    if let Some(fir) = segments.first() {
                        res_tree.root_mut().value().prepend(fir.to_owned().to_owned());
                    }
                    for (i, seg) in segments.into_iter().skip(1).enumerate() {
                        if i > 0 {
                            let clone = clone_to_root_in_tree(&mut res_tree, parent_element);
                            res_tree.get_mut(res).unwrap().append_id(clone);
                            res = clone;
                        }
                        res_tree.get_mut(res).unwrap().value().append(seg);
                    }
                };
            },
            SelectorPlaceholder => {
                if tree.get_text(child.id()) == "@at-root" {
                    todo!()
                }
            },
            ElementNameSelector => {
                let text = tree.get_text(child.id());
                res_tree.get_mut(res).unwrap().value().add_attrib("name", if text == "*" {"element".to_owned()} else {unescape(text)});
            },
            ClassSelector => {
                res_val_mut!().add_attrib("class", unescape(&tree.get_text(child.id())[1..]));
            },
            IdentifierSelector => {
                res_val_mut!().add_attrib("id", unescape(&tree.get_text(child.id())[1..]));
            },
            _BodyDeclaration(BodyDeclaration {
                declarations: _,
                body_decl_type: BodyDeclarationType::MixinDeclaration(_)
            }) => {
                let name = tree.get_text(child.value().node_type.unchecked_mixin_declaration_ref().identifier);
                res_val_mut!().add_attrib("class", name.to_owned());
            },
            PseudoSelector => {
                res_val_mut!().add_attrib(&unescape(tree.get_text(child.id())), "".to_owned())
            }
            AttributeSelector(selector) => {
                let identifier = tree.get_text(selector.identifier);

                let expression = tree.get_text(selector.value);
                let operator = tree.get_text(selector.operator);
                let value = match unescape(operator).as_str() {
                    "|=" => format!("{}-\u{2026}", Quotes::remove(&unescape(expression))), // exactly or followed by -words
                    "^=" => format!("{}\u{2026}", Quotes::remove(&unescape(expression))), // prefix
                    "$=" => format!("\u{2026}{}", Quotes::remove(&unescape(expression))), // suffix
                    "~=" => format!("\u{2026} {} \u{2026}", Quotes::remove(&unescape(expression))), // one of a list of words
                    "*=" => format!("\u{2026}{}\u{2026}", Quotes::remove(&unescape(expression))), // substring
                    _ => Quotes::remove(&unescape(expression)).to_owned()
                };
                res_val_mut!().add_attrib(&unescape(identifier), value);
            },
            _ => {}
        }
    }
    return res_tree
}

fn unescape(content: &str) -> String {
    let mut scanner = Scanner::default();
    scanner.set_source(content.to_owned());
    match scanner.scan_unquoted_string() {
        Some(token) => return token.text,
        None => content.to_owned()
    }
}

pub type SelectorPrinting = CssDataManager;

impl SelectorPrinting {
    pub fn selector_to_marked_string(&self, node_tree: &CssNodeTree, node_id: NodeId, flag_opts: Option<FlagOpts>) -> Vec<MarkedString> {
        let node = node_tree.get(node_id).unwrap();
        assert!(node.value().node_type.same_node_type(&CssNodeType::Selector));
        let ele_tree = selector_to_element(node_tree, node_id);
        let Some(ele_tree) = ele_tree else {
            return Vec::new();
        };
        let root = ele_tree.root().id();
        let mut marked_strings = MarkedStringPrinter::new("\"".to_owned()).print(ele_tree, root, flag_opts);
        marked_strings.push(self.selector_to_specificity_marked_string(node_tree, node_id));
        return marked_strings;
    }

    pub fn simple_selector_to_marked_string(&self, node_tree: &CssNodeTree, node_id: NodeId, flag_opts: Option<FlagOpts>) -> Vec<MarkedString> {
        assert!(node_tree.get(node_id).unwrap().value().node_type.same_node_type(&CssNodeType::SimpleSelector));
        let ele_tree = to_element(node_tree, node_id, None, None);
        let root = ele_tree.root().id();
        let mut marked_strings = MarkedStringPrinter::new("\"".to_owned()).print(ele_tree, root, flag_opts);
        marked_strings.push(self.selector_to_specificity_marked_string(node_tree, node_id));
        return marked_strings;
    }

    pub fn is_pseudo_element_identifier(&self, text: &str) -> bool {
        let reg = Regex::new("^::?([\\w-]+)").unwrap();
        let mut mat = reg.find_iter(text);
        if mat.next().is_none() {
            return false;
        }
        return self.get_pseudo_element(&("::".to_owned() + mat.next().unwrap().as_str())).is_some();
    }

    fn selector_to_specificity_marked_string(&self, node_tree: &CssNodeTree, node_id: NodeId) -> MarkedString {
        let specificity = self.calculate_score(node_tree, node_tree.get(node_id).unwrap());
        return MarkedString::String(format!("[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): ({}, {}, {})", specificity.id, specificity.attr, specificity.tag)); // TODO: i10n
    }

    fn calculate_most_specific_list_item (&self, node_tree: &CssNodeTree, child_nodes: Vec<NodeRef<CssNode>>) -> Specificity {
        // TODO: check why vscode has specificity variable here
        let mut most_specific_list_item = Specificity::default();
        for container_node in child_nodes {
            for child_node in container_node.children() {
                let item_specificity = self.calculate_score(node_tree, child_node);
                if item_specificity.id > most_specific_list_item.id {
                    most_specific_list_item = item_specificity;
                    continue;
                } else if item_specificity.id < most_specific_list_item.id {
                    continue;
                }

                if item_specificity.attr > most_specific_list_item.attr {
                    most_specific_list_item = item_specificity;
                    continue;
                } else if item_specificity.attr < most_specific_list_item.attr {
                    continue;
                }

                if item_specificity.tag > most_specific_list_item.tag {
                    most_specific_list_item = item_specificity;
                    continue;
                }
            }
        }
        return most_specific_list_item;
    }


    //https://www.w3.org/TR/selectors-3/#specificity
    fn calculate_score(&self, node_tree: &CssNodeTree, node: NodeRef<CssNode>) -> Specificity {
        let mut specificity = Specificity::default();
        for child in node.children() {
            use CssNodeType::*;
            match child.value().node_type {
                IdentifierSelector => specificity.id += 1,
                ClassSelector => {},
                AttributeSelector(..) => specificity.attr += 1,
                ElementNameSelector => {
                    if node_tree.get_text(child.id()) != "*" {
                        specificity.tag += 1;
                    }
                },
                PseudoSelector => {
                    let text = node_tree.get_text(child.id());
                    let grand_childs: Vec<NodeRef<CssNode>> = child.children().collect();

                    if self.is_pseudo_element_identifier(text) {
                        if text.to_lowercase().starts_with("::slotted") && grand_childs.len() > 0 {
                            // The specificity of ::slotted() is that of a pseudo-element, plus the specificity of its argument.
                            // ::slotted() does not allow a selector list as its argument, but this isn't the right place to give feedback on validity.
                            // Reporting the most specific child will be correct for correct CSS and will be forgiving in case of mistakes.
                            specificity.tag += 1;
                            specificity += self.calculate_most_specific_list_item(node_tree, grand_childs);
                            continue
                        }
                        specificity.tag += 1; // pseudo element
                        continue
                    } 

                    // where and child selectors zero specificity
                    if text.to_lowercase().starts_with(":where") {
                        continue
                    }
                    
                    // the most specific child selector
                    if RegexBuilder::new("^:(?:not|has|is)").case_insensitive(true).build().unwrap().is_match(text) && grand_childs.len() > 0 {
                        specificity += self.calculate_most_specific_list_item(node_tree, grand_childs);
                        continue
                    }

                    if RegexBuilder::new("^:(?:host|host-context)").case_insensitive(true).build().unwrap().is_match(text) && grand_childs.len() > 0 {
                        // The specificity of :host() is that of a pseudo-class, plus the specificity of its argument.
                        // The specificity of :host-context() is that of a pseudo-class, plus the specificity of its argument.
                        specificity.attr += 1;
                        specificity += self.calculate_most_specific_list_item(node_tree, grand_childs);
                        continue
                    }

                    if 
                        RegexBuilder::new("^:(?:nth-child|nth-last-child)").case_insensitive(true).build().unwrap().is_match(text) 
                        && grand_childs.len() > 0 
                    {
                        /* The specificity of the :nth-child(An+B [of S]?) pseudo-class is the specificity of a single pseudo-class plus, if S is specified, the specificity of the most specific complex selector in S */
                        // https://www.w3.org/TR/selectors-4/#the-nth-child-pseudo
                        specificity.attr += 1;
                        
                        if 
                            grand_childs.len() == 3 && 
                            grand_childs[1].value().node_type
                                .same_node_type(&BinaryExpression(crate::parser::css_node_types::BinaryExpression {
                                    left: node.id(),
                                    right: node.id(),
                                    operator: node.id(),
                                })
                        ) {
                            specificity += self.calculate_most_specific_list_item(node_tree, grand_childs[2].children().collect());
                            continue
                        }

                        // Edge case: 'n' without integer prefix A, with B integer non-existent, is not regarded as a binary expression token.
                        let pseudo_selector_text = node_tree.get_text(grand_childs[1].id());
                        let mut parser = crate::parser::css_parser::Parser::new_with_text(pseudo_selector_text.to_owned());
                        let first_token = parser.scanner.scan();
                        let second_token = parser.scanner.scan();
                        if first_token.text == "n" || (first_token.text == "-n" && second_token.text == "of") {
                            let mut complex_selector_list_nodes = Vec::new();
                            let complex_selector_text = &pseudo_selector_text[second_token.offset + 2..];
                            let complex_selector_array = complex_selector_text.split(",");
                            for _ in complex_selector_array {
                                if let Some(_) = parser.parse_node_by_fn(|p: &mut crate::parser::css_parser::Parser| p.parse_selector(false)) {
                                    complex_selector_list_nodes.push(parser.take_tree());
                                }
                            }

                            specificity += self.calculate_most_specific_list_item(
                                node_tree, 
                                complex_selector_list_nodes.iter().map(|t| t.0.root()).collect()
                            );
                            continue
                        }
                        continue
                    }

                    specificity.attr += 1; // pseudo class
                    continue
                },
                _ => {}
            }

            if child.children().count() > 0 {
                specificity += self.calculate_score(node_tree, child);
            }
        }
        return specificity;
    }


}

struct SelectorElementBuilder<'a> {
    node_tree: &'a CssNodeTree,
    ele_tree: &'a mut Tree<Element>,
    prev_node: Option<NodeId>,
    element: NodeId,
}

impl<'a> SelectorElementBuilder<'a> {
    pub fn new(node_tree: &'a CssNodeTree, ele_tree: &'a mut Tree<Element>, element: NodeId) -> SelectorElementBuilder<'a> {
        return Self {
            node_tree,
            ele_tree,
            prev_node: None,
            element,
        }
    }

    pub fn process_selector(&mut self, selector: NodeRef<CssNode>) {
        assert!(selector.value().node_type.same_node_type(&CssNodeType::Selector));
        let mut parent_element = None;

        if self.ele_tree.get(self.element).unwrap().parent().is_some() {
            if selector.children().any(|ch| ch.has_children() && ch.first_child().unwrap().value().node_type.same_node_type(&CssNodeType::SelectorCombinator)) {
                let curr = self.ele_tree.root();
                if curr.parent().is_some() && curr.parent().unwrap().parent().is_none() {
                    parent_element = Some(self.element);
                    self.element = curr.parent().unwrap().id();
                    self.ele_tree.get_mut(curr.id()).unwrap().detach();
                    self.prev_node = None;
                }
            }
        }

        for selector_child in selector.children() {
            use CssNodeType::*;
            if selector_child.value().node_type.same_node_type(&SimpleSelector) {
                if self.prev_node.is_some() && self.node_tree.get(self.prev_node.unwrap()).unwrap().value().node_type.same_node_type(&CssNodeType::SimpleSelector) {
                    let label_element = Element {
                        attributes: vec![Attribute {
                            name: "name".to_owned(), 
                            value: "\u{2026}".to_owned()
                        }],
                    };
                    let label_element_id = self.ele_tree.get_mut(self.element).unwrap().append(label_element).id();
                    self.element = label_element_id;
                } else if let Some(prev) = self.prev_node {
                    let prev_text = self.node_tree.get_text(prev);
                    if prev_text == "+" || prev_text == "~" {
                        if let Some(par) = self.ele_tree.get(self.element).unwrap().parent() {
                            self.element = par.id();
                        }
                    }
                }

                if let Some(prev) = self.prev_node {
                    if self.node_tree.get_text(prev) == "~" {
                        self.ele_tree.get_mut(self.element).unwrap().append(Element {
                            attributes: vec![
                                Attribute {
                                    name: "name".to_owned(),
                                    value: "\u{22EE}".to_owned(),
                                }
                            ]
                        });
                    }
                }

                let mut self_element = to_element(self.node_tree, selector_child.id(), Some(self.ele_tree), parent_element);
                let root = self_element.root().id();

                self.ele_tree.attach_tree(&mut self_element, self.element, root);
                self.element = root;
            }
            
            match selector_child.value().node_type {
                SimpleSelector | 
                SelectorCombinatorParent | 
                SelectorCombinatorShadowPiercingDescendant | 
                SelectorCombinatorSibling | 
                SelectorCombinatorAllSiblings => self.prev_node = Some(selector_child.id()),
                _ => {}
            }
        }
    }

}

fn is_new_selector_context(node: &CssNode) -> bool {
    return match &node.node_type {
        CssNodeType::_BodyDeclaration(b) => {
            match b.body_decl_type {
                BodyDeclarationType::MixinDeclaration(..) => true,
                _ => false
            }
        },
        CssNodeType::Stylesheet => true,
        _ => false
    }
}

fn selector_to_element(node_tree: &CssNodeTree, node_id: NodeId) -> Option<Tree<Element>> {
    macro_rules! node {() => {node_tree.get(node_id).unwrap()};}
    macro_rules! nodeval {() => {node_tree.get(node_id).unwrap().value()};}

    assert!(nodeval!().node_type.same_node_type(&CssNodeType::Selector));
    if node_tree.get_text(node_id) == "@at-root" {
        return None
    }

    let root = Element::default();
    let mut ele_tree = Tree::new(root);
    let mut parent_rule_sets = Vec::new();
    let rule_set = node!().parent().unwrap().id();

    let rule_set_dummy = CssNodeType::_BodyDeclaration(
        BodyDeclaration {
            declarations: None,
            body_decl_type: BodyDeclarationType::RuleSet(crate::parser::css_node_types::RuleSet {
                selectors: node_id,
            })
        }
    );

    if node_tree.get(rule_set).unwrap().value().node_type.same_node_type(&rule_set_dummy) {
        let mut parent = node_tree.get(rule_set).unwrap().parent(); // parent of the selector's ruleset
        while let Some(par) = parent {
            if is_new_selector_context(par.value()) {break}
            if par.value().node_type.same_node_type(&rule_set_dummy) {
                if node_tree.get_text(par.value().node_type.unchecked_rule_set_ref().selectors) == "@at-root" {
                    break;
                }
                parent_rule_sets.push(par.id());
            }
            parent = par.parent();
        }
    }

    let root = ele_tree.root().id();
    let mut builder = SelectorElementBuilder::new(&node_tree, &mut ele_tree, root);

    for rule_set in parent_rule_sets.into_iter().rev() {
        let selector = node_tree.get(
            node_tree.get(rule_set).unwrap().value().node_type.unchecked_rule_set_ref().selectors
        ).unwrap().first_child();
        if let Some(sel) = selector {
            builder.process_selector(sel);
        }
    }

    builder.process_selector(node_tree.get(node_id).unwrap());
    return Some(ele_tree);

}



