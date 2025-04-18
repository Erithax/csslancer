
use std::cmp::Ordering;

use ego_tree::{NodeId, NodeRef, Tree};
use lsp_types::{LanguageString, MarkedString};
use regex::{Regex, RegexBuilder};
use rowan::SyntaxNode;

use crate::row_parser::ast::AstNode;
use crate::row_parser::nodes_types::{CssLanguage, SyntaxToken};
use crate::tokenizer::extra::unescape;
use crate::ext::TreeAttach;
use crate::data::data_manager::CssDataManager;
use crate::row_parser::{
    syntax_kind_gen::SyntaxKind,
    nodes_gen,
};

use super::hover::FlagOpts;


#[derive(Debug, Clone, PartialEq, Eq)]
struct Attribute {
    name: String,
    value: String,
}

#[derive(Debug, Clone, Default)]
pub struct Element {
    attributes: Vec<Attribute>,
}

type ElementTree = ego_tree::Tree<Element>;

impl Element {
    fn new_label(text: &str) -> Self {
        Element {
            attributes: vec![Attribute {
                name: "name".to_owned(), 
                value: text.to_owned() 
            }]
        }
    }

    fn get_attribute_mut(&mut self, name: &str) -> Option<&mut Attribute> {
        for attribute in self.attributes.iter_mut() {
            if attribute.name == name {
                return Some(attribute)
            }
        }
        None
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
        None
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

    fn add_attr(&mut self, attr: Attribute) {
        if let Some(self_attr) = self.get_attribute_mut(&attr.name) {
            self_attr.value += " ";
            self_attr.value += &attr.value;
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
        Self {
            result: Vec::new(),
            quote,
        }
    }

    pub fn print(&mut self, tree: ElementTree, ele_id: NodeId, flag_opts: Option<FlagOpts>) -> Vec<MarkedString> {
        let element = tree.get(ele_id).unwrap();
        if element.id() == tree.root().id() {
            if element.has_children() {
                self.do_print(element.children(), 0);
            } else {
                self.do_print([element], 0);
            }
        } else {
            self.do_print([element], 0);
        }
        let value = match flag_opts {
            Some(fo) => format!("{}\n … ", fo.text) + &self.result.join("\n"),
            None => self.result.join("\n")
        };
        vec![MarkedString::LanguageString(LanguageString {
            language: "html".to_string(),
            value,
        })]
    }

    fn do_print<'a>(&mut self, nodes: impl IntoIterator<Item = NodeRef<'a, Element>>, indent: usize) {
        for node in nodes {
            self.do_print_ele(node, indent);
            if node.has_children() {
                self.do_print(node.children(), indent + 1);
            }
        }
    }

    fn write_line(&mut self, level: usize, content: &str) {
        let ident = "  ".repeat(level);
        self.result.push(ident + content);
    }

    fn do_print_ele(&mut self, node: NodeRef<Element>, indent: usize) {
        let element = node.value();
        let name = element.get_value_ref("name");

        // special case: a simple label
        if let Some(name) = name {
            if name == "\u{2026}" || name == "\u{22EE}" {
                self.write_line(indent, name);
                return
            }
        }

        let mut content = "<".to_string();

        // element name
        if let Some(n) = name {
            content += n;
        } else {
            content += "element";
        }


        // attributes
        for attr in element.attributes.iter() {
            if attr.name != "name" {
                content += " ";
                content += &attr.name;
                if !attr.value.is_empty() { // TODO make attribute.value Option<String> to handle this better
                    content += "=";
                    Quotes::ensure(&mut content, &attr.value, &self.quote);
                }
            }
        }

        content += ">";
        self.write_line(indent, &content);

    }

}

struct Quotes;
impl Quotes {
    pub fn ensure(content: &mut String, value: &str, which: &str) {
        content.push_str(which);
        content.push_str(Self::remove(value));
        content.push_str(which);
    }

    // removes the first and last character of a string if they are both quotes
    pub fn remove(value: &str) -> &str {
        let reg = Regex::new(r#"^['"](.*)['"]$"#).unwrap();
        if reg.is_match(value) {
            return &value[1..value.len()-1]
        }
        value
    }
}

#[derive(Debug, Default)]
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
        self
    }
}

impl std::ops::AddAssign for Specificity {
    fn add_assign(&mut self, rhs: Self) {
        self.id += rhs.id;
        self.attr += rhs.attr;
        self.tag += rhs.tag;
    }
}

// clones node and all ancestors, returns NodeId of `node_id` in the new tree
fn clone_to_root(ele_tree: &Tree<Element>, node_id: NodeId) -> (Tree<Element>, NodeId) {
    let mut res = Tree::new(ele_tree.root().value().clone());
    let root_id = res.root().id();

    let mut curr_node = res.orphan(ele_tree.get(node_id).unwrap().value().clone()).id();
    let new_node_id = curr_node;

    while let Some(par) = res.get(curr_node).unwrap().parent() {
        if par.id() == root_id {break}
        let par_clone = par.value().clone();
        curr_node = par.id();
        let mut next = res.orphan(par_clone);
        next.append_id(curr_node);
    }

    res.root_mut().append_id(curr_node);
    (res, new_node_id)
}

/// Odd function
// fn clone_to_root_in_tree(ele_tree: &mut Tree<Element>, node_id: NodeId) -> NodeId {
//     let root_id = ele_tree.root().id();

//     let mut curr_orig_node_id = node_id;
//     let mut curr_node_id = ele_tree.orphan(ele_tree.get(node_id).unwrap().value().clone()).id();

//     while let Some(par) = ele_tree.get(curr_orig_node_id).unwrap().parent() {
//         if par.id() == root_id {break}
//         curr_orig_node_id = par.id();
//         let mut next: ego_tree::NodeMut<'_, Element> = ele_tree.orphan(par.value().clone());
//         next.append_id(curr_node_id);
//         curr_node_id = next.id();
//     }

//     return curr_node_id
// }

// NON-recursively converts SimpleSelector node with `node_id` in `node_tree` to an `Tree<Element>`
fn to_element(syntax_node: &nodes_gen::SimpleSelector, ele_tree: Option<&Tree<Element>>, parent_ele_id: Option<NodeId>) -> Tree<Element> {
    assert!(ele_tree.is_none() == parent_ele_id.is_none());
    // assert!(node_tree.0.0.root().children().count() == 1);
    //assert!(syntax_node.ancestors().last().unwrap().children().count() == 1);

    //let node = node_tree.get(node_id).unwrap();
    
    let mut res_tree = Tree::new(Element {
        attributes: Vec::new(),
    });
    let mut res: NodeId = res_tree.root().id();

    macro_rules! res_val_mut {
        () => {
            res_tree.get_mut(res).unwrap().value()
        };
    }
    
    for child in syntax_node.syntax.children() {
        use SyntaxKind::*;
        match child.kind() {
            SELECTOR_COMBINATOR => {
                if let (Some(ele_tree), Some(parent_element)) = (&ele_tree, parent_ele_id) {
                    let text = child.text().to_string();
                    let segments: Vec<&str> = text.split('&').collect();
                    
                    debug_assert!(segments.len() != 1);
                    if segments.len() == 1 {
                        // should not happen
                        res_val_mut!().add_attr(Attribute {name: "name".to_owned(), value: segments.first().unwrap_or(&"<error>").to_owned().to_owned()});
                        continue
                    }

                    let (clone_parent_to_root_tree, clone_parent_id) = clone_to_root(ele_tree, parent_element);
                    res_tree = clone_parent_to_root_tree;
                    res = clone_parent_id;

                    if let Some(fir) = segments.first() {
                        res_tree.root_mut().value().prepend(fir.to_owned().to_owned());
                    }
                    for (i, seg) in segments.into_iter().skip(1).enumerate() {
                        if i > 0 {
                            let (mut clone_parent_to_root_tree, clone_parent_id) = clone_to_root(&res_tree, parent_element);
                            let clone_parent_to_root_tree_root = clone_parent_to_root_tree.root().id();
                            res_tree.attach_tree(&mut clone_parent_to_root_tree, res, clone_parent_to_root_tree_root);
                            res = clone_parent_id;
                        }
                        res_val_mut!().append(seg);
                    }
                };
            },
            SCSS_SELECTOR_PLACEHOLDER => {
                if child.text() == "@at-root" {
                    todo!()
                }
            },
            SELECTOR_ELEMENT_NAME => {
                let text = child.text();
                res_val_mut!().add_attrib("name", if text == "*" {"element".to_owned()} else {unescape(&text.to_string())});
            },
            SELECTOR_CLASS => {
                res_val_mut!().add_attrib("class", unescape(&child.text().to_string()[1..]));
            },
            SELECTOR_IDENTIFIER => {
                res_val_mut!().add_attrib("id", unescape(&child.text().to_string()[1..]));
            },
            XCSS_MIXIN_DECLARATION => {
                todo!(r#"let typed = nodes_gen::XcssMixinDeclaration::cast(XCSS_MIXIN_DECLARATION);
                let identifier = typed.get_identifier();
                let name = identifier.text();
                res_val_mut!().add_attrib("class", name.to_owned());"#)
            },
            SELECTOR_PSEUDO => {
                res_val_mut!().add_attrib(&unescape(&child.text().to_string()), "".to_owned())
            }
            SELECTOR_ATTRIBUTE => {
                let typed = nodes_gen::SelectorAttribute::cast(child).unwrap();

                let identifier = typed.identifier_token().unwrap().text().to_string();
                let value = if let Some(expression) = typed.binary_expression() {
                    let expr_unesc_text = unescape(&expression.syntax.text().to_string());
                    let expression_text = Quotes::remove(&expr_unesc_text);
                    if let Some(operator) = typed.operator() {
                        match operator.syntax.text().to_string().as_str() {
                            "|=" => format!("{}-\u{2026}", expression_text), // exactly or followed by -words
                            "~=" => format!(" \u{2026} {} \u{2026} ", expression_text), // one of a list of words
                            "^=" => format!("{}\u{2026}", expression_text), // prefix
                            "$=" => format!("\u{2026}{}", expression_text), // suffix
                            "*=" => format!("\u{2026}{}\u{2026}", expression_text), // substring
                            "=" => expression_text.to_owned(),
                            _ => "<unknown attribute operator>".to_owned(),
                        }
                    } else {
                        String::new()
                    }
                } else {
                    "undefined".to_owned()
                };
                
                res_val_mut!().add_attrib(&unescape(&identifier), value);
            },
            _ => {}
        }
    }
    res_tree
}


pub type SelectorPrinting = CssDataManager;

impl SelectorPrinting {
    pub fn selector_to_marked_string(&self, selector: &nodes_gen::Selector, flag_opts: Option<FlagOpts>) -> Vec<MarkedString> {
        let ele_tree = selector_to_element(selector);
        let Some(ele_tree) = ele_tree else {
            return Vec::new();
        };
        let root = ele_tree.root().id();
        //let root = ele_tree.root().first_child().unwrap().id(); TODO
        let mut marked_strings = MarkedStringPrinter::new("\"".to_owned()).print(ele_tree, root, flag_opts);
        marked_strings.push(self.selector_to_specificity_marked_string(&selector.syntax));
        marked_strings
    }

    pub fn simple_selector_to_marked_string(&self, simple_selector: &nodes_gen::SimpleSelector, flag_opts: Option<FlagOpts>) -> Vec<MarkedString> {
        let ele_tree = to_element(simple_selector, None, None);
        let root = ele_tree.root().id();
        let mut marked_strings = MarkedStringPrinter::new("\"".to_owned()).print(ele_tree, root, flag_opts);
        marked_strings.push(self.selector_to_specificity_marked_string(&simple_selector.syntax));
        marked_strings
    }

    pub fn is_pseudo_element_identifier(&self, text: &str) -> bool {
        let reg = Regex::new(r"^::?(?<ident>[\w-]+)").unwrap();
        let captures = reg.captures(text);
        if let Some(Some(ident)) = captures.map(|c| c.name("ident")) {
            self.get_pseudo_element(&("::".to_owned() + ident.as_str())).is_some()
        } else {
            false
        }
    }

    fn selector_to_specificity_marked_string(&self, selector: &SyntaxNode<CssLanguage>) -> MarkedString {
        assert!(matches!(selector.kind(), SyntaxKind::SELECTOR | SyntaxKind::SIMPLE_SELECTOR));
        let specificity = self.calculate_score(selector);
        MarkedString::String(format!("[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): ({}, {}, {})", specificity.id, specificity.attr, specificity.tag)) // TODO: i10n
    }

    fn calculate_most_specific_list_item (&self, child_nodes: impl IntoIterator<Item = SyntaxNode<CssLanguage>>) -> Specificity {
        // TODO: check why vscode has specificity variable here
        let mut most_specific_list_item = Specificity::default();
        for container_node in child_nodes {
            // simple selector
            for child_node in container_node.children() {
                let item_specificity = self.calculate_score(&child_node);

                match item_specificity.id.cmp(&most_specific_list_item.id) {
                    Ordering::Greater => {most_specific_list_item = item_specificity; continue},
                    Ordering::Less => continue,
                    Ordering::Equal => {}
                }

                match item_specificity.attr.cmp(&most_specific_list_item.attr) {
                    Ordering::Greater => {most_specific_list_item = item_specificity; continue},
                    Ordering::Less => continue,
                    Ordering::Equal => {}
                }

                if item_specificity.tag > most_specific_list_item.tag {
                    most_specific_list_item = item_specificity;
                    continue;
                }
            }
        }
        most_specific_list_item
    }


    //https://www.w3.org/TR/selectors-3/#specificity
    fn calculate_score(&self, selector: &SyntaxNode<CssLanguage>) -> Specificity {
        //assert!(matches!(selector.kind(), SyntaxKind::SELECTOR | SyntaxKind::SIMPLE_SELECTOR | SyntaxKind::SELECTOR_ATTRIBUTE), "was {:?}", selector.kind());
        let mut specificity = Specificity::default();
        for child in selector.children() {
            use SyntaxKind::*;
            match child.kind() {
                SELECTOR_IDENTIFIER => specificity.id += 1,
                SELECTOR_CLASS | SELECTOR_ATTRIBUTE => specificity.attr += 1,
                SELECTOR_ELEMENT_NAME => {
                    if child.text() != "*" {
                        specificity.tag += 1;
                    }
                },
                SELECTOR_PSEUDO => {
                    let text = &child.text().to_string();
                    let grand_childs: Vec<SyntaxNode<CssLanguage>> = child.children().collect();

                    if self.is_pseudo_element_identifier(text) {
                        if text.to_lowercase().starts_with("::slotted") && !grand_childs.is_empty() {
                            // The specificity of ::slotted() is that of a pseudo-element, plus the specificity of its argument.
                            // ::slotted() does not allow a selector list as its argument, but this isn't the right place to give feedback on validity.
                            // Reporting the most specific child will be correct for correct CSS and will be forgiving in case of mistakes.
                            specificity.tag += 1;
                            specificity += self.calculate_most_specific_list_item(grand_childs);
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
                    if RegexBuilder::new("^:(?:not|has|is)").case_insensitive(true).build().unwrap().is_match(text) && !grand_childs.is_empty() {
                        specificity += self.calculate_most_specific_list_item(grand_childs);
                        continue
                    }

                    if RegexBuilder::new("^:(?:host|host-context)").case_insensitive(true).build().unwrap().is_match(text) && !grand_childs.is_empty() {
                        // The specificity of :host() is that of a pseudo-class, plus the specificity of its argument.
                        // The specificity of :host-context() is that of a pseudo-class, plus the specificity of its argument.
                        specificity.attr += 1;
                        specificity += self.calculate_most_specific_list_item(grand_childs);
                        continue
                    }

                    if 
                        RegexBuilder::new("^:(?:nth-child|nth-last-child)").case_insensitive(true).build().unwrap().is_match(text) 
                        && !grand_childs.is_empty()
                    {
                        /* The specificity of the :nth-child(An+B [of S]?) pseudo-class is the specificity of a single pseudo-class plus, if S is specified, the specificity of the most specific complex selector in S */
                        // https://www.w3.org/TR/selectors-4/#the-nth-child-pseudo
                        specificity.attr += 1;
                        
                        if 
                            grand_childs.len() == 3 && 
                            grand_childs[1].kind() == SyntaxKind::BINARY_EXPRESSION {
                            specificity += self.calculate_most_specific_list_item(grand_childs[2].children());
                            continue
                        }
                        println!("grand childs {}", grand_childs.iter().fold(String::new(), |acc, nex| acc + ", " + &nex.to_string()));

                        // let first_token = child.first_token().and_then(|t| t.next_token()).and_then(|t| t.next_token());
                        // let second_token = first_token.as_ref().and_then(|f| f.next_token());
                        // let third_token = second_token.as_ref().and_then(|t| t.next_token());
                        // let first_token_text = first_token.map(|f| f.text().to_string()).unwrap_or(String::new());
                        // let second_token_text = second_token.map(|s| s.text().to_string()).unwrap_or(String::new());
                        // let third_token_text = third_token.map(|t| t.text().to_string()).unwrap_or(String::new());

                        fn is_an(sn: &SyntaxToken) -> bool {
                            let k = sn.kind();
                            k == SyntaxKind::CXDIM_AN_PLUS_B || k == SyntaxKind::CXID_AN_PLUS_B_SYNTAX_AN
                        }

                        let mut is_an_plus_b = false;
                        child.first_token()
                            .as_ref().and_then(|t| {is_an_plus_b = is_an_plus_b || is_an(t); t.next_token()})
                            .as_ref().and_then(|t| {is_an_plus_b = is_an_plus_b || is_an(t); t.next_token()})
                            .as_ref().map(|t| {is_an_plus_b = is_an_plus_b || is_an(t); t});

                        println!("is_an_plus_b: {is_an_plus_b}");
                        if is_an_plus_b {
                            let selector_list = grand_childs.iter().find(|g| g.kind() == SyntaxKind::UNDEFINED);
                            println!("before {:?}", specificity);
                            specificity += selector_list
                                .map(|sl| 
                                    self.calculate_most_specific_list_item(sl.children().filter(|s| s.kind() == SyntaxKind::SELECTOR))
                                ).unwrap_or(Specificity::default());
                            println!("after  {:?}", specificity);
                        }
                        continue

                        // Edge case: 'n' without integer prefix A, with B integer non-existent, is not regarded as a binary expression token.
                        //let pseudo_selector_text = grand_childs[1].text().to_string();
                        //let mut parser = crate::parser::css_parser::Parser::new_with_text(pseudo_selector_text.to_owned());
                        // let first_token = parser.token.clone();
                        // let second_token = parser.scanner.scan();
                        // if first_token.text == "n" || (first_token.text == "-n" && second_token.text == "of") {
                        //     parser.prev_token = Some(second_token.clone());
                        //     parser.token = parser.scanner.scan(); 
                        //     for _ in pseudo_selector_text[second_token.offset + 2..].split(",") {
                        //         if let Some(n) = parser.parse_node_by_fn(|p: &mut crate::parser::css_parser::Parser| p.parse_selector(false)) {
                        //             parser.tree.0.root_mut().append_id(n);
                        //         }
                        //         if parser.token.token_type == crate::parser::css_scanner::TokenType::Comma {
                        //             parser.consume_token();
                        //         }
                        //     }
                        //     let css_node_tree: CssNodeTree = parser.into_css_node_tree();
                        //     specificity += self.calculate_most_specific_list_item(
                        //         &css_node_tree,
                        //         css_node_tree.0.0.root().children().into_iter().collect(), 
                        //     );
                        //     continue
                        // }
                        // continue
                    }

                    specificity.attr += 1; // pseudo class
                    continue
                },
                _ => {}
            }

            if child.children().count() > 0 {
                specificity += self.calculate_score(&child);
            }
        }
        specificity
    }


}

struct SelectorElementBuilder<'a> {
    ele_tree: &'a mut Tree<Element>,
    prev_node: Option<SyntaxNode<CssLanguage>>,
    element: NodeId,
}

impl<'a> SelectorElementBuilder<'a> {
    pub fn new(ele_tree: &'a mut Tree<Element>, element: NodeId) -> SelectorElementBuilder<'a> {
        Self {
            ele_tree,
            prev_node: None,
            element,
        }
    }

    // Processes node of type 'CssNodeType::Selector` `selector`
    #[allow(clippy::collapsible_if)]
    pub fn process_selector(&mut self, selector: &nodes_gen::Selector) {
        let selector = selector.syntax();
        let mut parent_element = None;

        if self.ele_tree.get(self.element).unwrap().parent().is_some() {
            if selector.children().any(|ch| ch.first_child().is_some_and(|grand_ch| grand_ch.kind() == SyntaxKind::SELECTOR_COMBINATOR)) {
                let curr = self.ele_tree.root();
                if curr.parent().is_some_and(|p| p.id() == curr.tree().root().id()) {
                    parent_element = Some(self.element);
                    self.element = curr.parent().unwrap().id();
                    self.ele_tree.get_mut(curr.id()).unwrap().detach();
                    self.prev_node = None;
                }
            }
        }

        for mut selector_child in selector.children() {
            if selector_child.kind() == SyntaxKind::SIMPLE_SELECTOR {                
                if let Some(prev) = &self.prev_node {
                    // we go deeper in the tree
                    if prev.kind() == SyntaxKind::SIMPLE_SELECTOR {
                        // descendant combinator ' ' (whitespace)
                        self.element = self.ele_tree.get_mut(self.element).unwrap().append(Element::new_label("\u{2026}")).id(); // horizontal elipses …
                    } else {
                        let prev_text = prev.text();
                        if prev_text == "+" || prev_text == "~" {
                            // sibling combinator
                            if let Some(par) = self.ele_tree.get(self.element).unwrap().parent() {
                                self.element = par.id();
                            }
                        }
                    }
    
                    if prev.text() == "~" {
                        self.ele_tree.get_mut(self.element).unwrap().append(Element::new_label("\u{22EE}")).id(); // vertical elipses '⋮'
                    }
                }

                let typed = nodes_gen::SimpleSelector::cast(selector_child).unwrap();
                let mut self_element = to_element(&typed, parent_element.map(|_| &*self.ele_tree), parent_element);
                selector_child = typed.syntax;
                let self_element_root = self_element.root().id();

                self.element = self.ele_tree.attach_tree(&mut self_element, self.element, self_element_root);
            }
            
            match selector_child.kind() {
                SyntaxKind::SIMPLE_SELECTOR | 
                SyntaxKind::SELECTOR_COMBINATOR_PARENT | 
                SyntaxKind::SELECTOR_COMBINATOR_SHADOW_PIERCING_DESCENDANT | 
                SyntaxKind::SELECTOR_COMBINATOR_SIBLING | 
                SyntaxKind::SELECTOR_COMBINATOR_ALL_SIBLINGS => self.prev_node = Some(selector_child),
                _ => {}
            }
        }
    }

}

fn is_new_selector_context(node: &SyntaxNode<CssLanguage>) -> bool {
    matches!(node.kind(), SyntaxKind::XCSS_MIXIN_DECLARATION | SyntaxKind::SOURCE_FILE)
}

/// Creates `Tree<Element>` for CssNodeType::Selector at `node_id` in `node_tree`, searching upwards of `node_id` for parent rulesets
fn selector_to_element(typed_node: &nodes_gen::Selector) -> Option<Tree<Element>> {
    let syntax_node = typed_node.syntax();
    
    if syntax_node.text() == "@at-root" {
        return None
    }

    let mut parent_rule_sets: Vec<nodes_gen::RuleSet> = Vec::new();
    let maybe_rule_set = syntax_node.parent();

    if let Some(maybe_rule_set) = maybe_rule_set {
        if maybe_rule_set.kind() == SyntaxKind::RULE_SET {
            let mut parent = maybe_rule_set.parent(); // parent of the selector's ruleset
            while let Some(par) = parent {
                if is_new_selector_context(&par) {break}
                if let Some(typed) = nodes_gen::RuleSet::cast(par.clone()) {
                    let selectors = typed.selectors();
                    if selectors.into_iter().fold(String::new(), |acc, nex| acc + &nex.syntax.text().to_string()) == "@at-root" {{
                        break;
                    }}
                    parent_rule_sets.push(typed);
                }
                parent = par.parent();
            }
        }
    }

    let mut ele_tree = Tree::new(Element::default());
    let ele_tree_root = ele_tree.root().id();
    let mut builder = SelectorElementBuilder::new(&mut ele_tree, ele_tree_root); 
    for rule_set in parent_rule_sets.into_iter().rev() { 
        let mut selectors = rule_set.selectors();
        
        if let Some(sel) = selectors.next() {
            builder.process_selector(&sel);
        }
    }

    builder.process_selector(typed_node);
    Some(ele_tree)
}








#[cfg(test)]
mod selector_printing_test {

    use ego_tree::NodeRef;
    use lsp_types::{LanguageString, MarkedString, Url};
    use rowan::{TextSize, TokenAtOffset};

    use crate::row_parser::{
        ast::AstNode, nodes_gen, 
        nodes_types::SyntaxNode, 
        syntax_kind_gen::SyntaxKind,
        parser::Parser,
    };
    use crate::services::selector_printing::{self, Attribute};
    use crate::workspace::source::Source;

    fn element_to_string(element: NodeRef<selector_printing::Element>) -> String {
        let mut label = element.value().get_value_ref("name").unwrap_or("").to_owned();
        let attributes: Vec<&selector_printing::Attribute> = element.value().attributes.iter().filter(|a| a.name != "name").collect();
        if attributes.len() > 0 {
            label += "[";
            let mut needs_seperator = false;
            for attribute in attributes {
                if attribute.name != "name" {
                     if needs_seperator {
                        label += "|";
                    }
                    needs_seperator = true;
                    label = label + &attribute.name + "=" + &attribute.value;
                }
            }
            label += "]";
        }

        let mut children = element.children();
        if let Some(first_child)  = children.next() {
            label += "{";
            label += &element_to_string(first_child);
            for child in children {
                label += "|";
                label += &element_to_string(child);
            }
            label += "}";
        }   
        
        return label;
    }

    fn do_parse(input: String, selector_name: String) -> Option<(Source, nodes_gen::Selector)> {
        let source = Source::new(Url::parse("test://test/test.css").unwrap(), &input, 0);

        println!("{}", source.parse.fancy_string());

        let node = match source.parse.syntax_node().token_at_offset(TextSize::new(input.find(&selector_name).unwrap() as u32)) {
            TokenAtOffset::Single(t) => t,
            TokenAtOffset::Between(a, b) => if a.kind().is_trivia() && !b.kind().is_trivia() {b} else if !a.kind().is_trivia() && b.kind().is_trivia() {a} else {a},
            TokenAtOffset::None => panic!("no bueno"),
        }
            .parent_ancestors()
            .find(|a| a.kind() == SyntaxKind::SELECTOR)?;

        let node = nodes_gen::Selector::cast(node).unwrap();
        
        return Some((source, node));
    }

    fn assert_selector(input: &str, selector_name: &str, expected: &str) {
        let src_and_selector = do_parse(input.to_owned(), selector_name.to_owned());
        assert!(src_and_selector.is_some());
        let (_source, selector) = src_and_selector.unwrap();
        //source.tree.0.assert_valid();
        //let s = source.tree.fancy_string();
        //println!("{s}");
        let element: Option<ego_tree::Tree<selector_printing::Element>> = selector_printing::selector_to_element(&selector);
        assert!(element.is_some());
        let element = element.unwrap();

        assert_eq!(element_to_string(element.root()), expected);
    }

    pub struct ExpectedElement<'a> {
        name: &'a str,
        value: &'a str
    }

    fn assert_element(input: &str, expected: &[ExpectedElement]) {
        println!("{input}");
        let expected = expected.into_iter().map(|e| Attribute {name: e.name.to_owned(), value: e.value.to_owned()}).collect::<Vec<Attribute>>();
        // let p: Parser = Parser::new_with_text(input.to_owned());
        // let node = p.into_parsed_by_fn(Parser::parse_simple_selector);

        let (success, (green, _errors)) = crate::row_parser::must_parse_text_as_fn(
            input, 
            |p: &mut Parser| p.parse_simple_selector());
        let root = SyntaxNode::new_root(green.clone());

        assert!(success);

        let root = nodes_gen::SimpleSelector::cast(root).unwrap();

        let actual = selector_printing::to_element(&root, None, None);
        let actual = &actual.root().value().attributes;

        assert_eq!(actual, &expected);
    }

    fn assert_selector_markdown(input: &str, selector_name: &str, expected: &[MarkedString]) {
        println!("input: {input}, selector_name: {selector_name}, expected: {expected:?}");
        let tree_and_selector = do_parse(input.to_owned(), selector_name.to_owned());
        assert!(tree_and_selector.is_some());
        let (_source, selector) = tree_and_selector.unwrap();
        let selector_printer = selector_printing::SelectorPrinting::new(true, None);
        let printed_element = selector_printer.selector_to_marked_string(&selector, None);

        assert_eq!(printed_element, expected);
    }

    pub struct BorrowLangString<'a> {
        language: &'a str,
        value: &'a str,
    }
    impl From<BorrowLangString<'_>> for MarkedString {
        fn from(value: BorrowLangString<'_>) -> Self {
            MarkedString::LanguageString(LanguageString{language: value.language.to_owned(), value: value.value.to_owned()})
        }
    }

    fn assert_selector_markdown_standard(input: &str, selector_name: &str, expected: (BorrowLangString, &str)) {
        println!("assert_selector_markdown_standard()");
        let expected = &[
            MarkedString::LanguageString(
                LanguageString {
                    language: expected.0.language.to_owned(),
                    value: expected.0.value.to_owned(),
                }
            ),
            MarkedString::String(expected.1.to_owned()),
        ];
        assert_selector_markdown(input, selector_name, expected);
    }

    // =======================
    // CSS - Selector Printing
    // =======================

    #[test]
    fn class_hash_elename_attr() {
        assert_element("element", &[ExpectedElement { name: "name", value: "element" }]);
        assert_element(".div", &[ExpectedElement { name: "class", value: "div" }]);
        assert_element("#first", &[ExpectedElement{ name: "id", value: "first" }]);
        assert_element("element.on", &[
            ExpectedElement{ name: "name", value: "element" }, 
            ExpectedElement{ name: "class", value: "on" }
        ]);
        assert_element("element.on#first", &[
            ExpectedElement{ name: "name", value: "element" },
            ExpectedElement{ name: "class", value: "on" },
            ExpectedElement{ name: "id", value: "first" }
        ]);
        assert_element(".on#first", &[
            ExpectedElement{ name: "class", value: "on" }, 
            ExpectedElement{ name: "id", value: "first" }
        ]);

        assert_element("[lang='de']", &[ExpectedElement{ name: "lang", value: "de" }]);
        // TODO assert_element("[enabled]", &[ExpectedElement{ name: "enabled", value: "" }]);
    }

    #[test]
    fn simple_selector() {
        assert_selector("element { }", "element", "{element}");
        assert_selector("element.div { }", "element", "{element[class=div]}");
        assert_selector("element.on#first { }", "element", "{element[class=on|id=first]}");
        assert_selector("element:hover { }", "element", "{element[:hover=]}");
        assert_selector("element[lang=\"de\"] { }", "element", "{element[lang=de]}");
        assert_selector("element[enabled] { }", "element", "{element[enabled=undefined]}");
        assert_selector("element[foo~=\"warning\"] { }", "element", "{element[foo= … warning … ]}");
        assert_selector("element[lang|=\"en\"] { }", "element", "{element[lang=en-…]}");
        assert_selector("* { }", "*", "{element}");
    }

    #[test]
    fn selector() {
        assert_selector("e1 e2 { }", "e1", "{e1{…{e2}}}");
        assert_selector("e1 .div { }", "e1", "{e1{…{[class=div]}}}");
        assert_selector("e1 > e2 { }", "e2", "{e1{e2}}");
        assert_selector("e1, e2 { }", "e1", "{e1}"); //pass
        assert_selector("e1 + e2 { }", "e2", "{e1|e2}"); //pass
        assert_selector("e1 ~ e2 { }", "e2", "{e1|⋮|e2}"); //pass
    }

    #[test]
    fn escaping() {
        assert_selector("#\\34 04-error { }", "#\\34 04-error", "{[id=404-error]}");
    }


    // ===================================
    // CSS - MarkedStringPrinter selectors
    // ===================================
    
    #[test]
    fn descendant_selector() {
        assert_selector_markdown_standard("e1 e2 { }", "e1", (
            BorrowLangString { language: "html", value: "<e1>\n  …\n    <e2>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 2)"
        ));
        assert_selector_markdown_standard("e1 .div { }", "e1", (
            BorrowLangString { language: "html", value: "<e1>\n  …\n    <element class=\"div\">" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 1, 1)"
        ));
    }

    #[test]
    fn child_selector() {
        assert_selector_markdown_standard("e1 > e2 { }", "e2", (
            BorrowLangString { language: "html", value: "<e1>\n  <e2>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 2)"
        ));
    }

    #[test]
    fn group_selector() {
        assert_selector_markdown_standard("e1, e2 { }", "e1", (
            BorrowLangString { language: "html", value: "<e1>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 1)"
        ));
        assert_selector_markdown_standard("e1, e2 { }", "e2", (
            BorrowLangString { language: "html", value: "<e2>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 1)"
        ));
    }

    #[test]
    fn sibling_selector() {
        assert_selector_markdown_standard("e1 + e2 { }", "e2", (
            BorrowLangString { language: "html", value: "<e1>\n<e2>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 2)"
        ));
        assert_selector_markdown_standard("e1 ~ e2 { }", "e2", (
            BorrowLangString { language: "html", value: "<e1>\n⋮\n<e2>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 2)"
        ));
    }

    // =================================================
    // CSS - MarkedStringPrinter selectors specificities
    // =================================================

    //
    #[test]
    fn attribute_selector() {
        assert_selector_markdown_standard("h1 + *[rel=up]", "h1", (
            BorrowLangString { language: "html", value: "<h1>\n<element rel=\"up\">" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 1, 1)"
        ));
    }

    #[test]
    fn class_selector() {
        assert_selector_markdown_standard("ul ol li.red", "ul", (
            BorrowLangString { language: "html", value: "<ul>\n  …\n    <ol>\n      …\n        <li class=\"red\">" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 1, 3)"
        ));
        assert_selector_markdown_standard("li.red.level", "li", (
            BorrowLangString { language: "html", value: "<li class=\"red level\">" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 2, 1)"
        ));
    }

    #[test]
    fn pseudo_class_selector() {
        assert_selector_markdown_standard("p:focus", "p", (
            BorrowLangString { language: "html", value: "<p :focus>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 1, 1)"
        ));
    }

    #[test]
    fn element_selector() {
        assert_selector_markdown_standard("li", "li", (
            BorrowLangString { language: "html", value: "<li>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 1)"
        ));
        assert_selector_markdown_standard("ul li", "ul", (
            BorrowLangString { language: "html", value: "<ul>\n  …\n    <li>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 2)"
        ));
        assert_selector_markdown_standard("ul ol+li", "ul", (
            BorrowLangString { language: "html", value: "<ul>\n  …\n    <ol>\n    <li>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 3)"
        ));
    }

    #[test]
    fn pseudo_element_selector() {
        assert_selector_markdown_standard("p::after", "p", (
            BorrowLangString { language: "html", value: "<p ::after>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 2)"
        ));
        assert_selector_markdown_standard("p:after", "p", (
            BorrowLangString { language: "html", value: "<p :after>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 2)"
        ));
    }

    #[test]
    fn identifier_selector() {
        assert_selector_markdown_standard("#x34y", "#x34y", (
            BorrowLangString { language: "html", value: "<element id=\"x34y\">" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 0, 0)"
        ));
    }

    #[test]
    fn ignore_universal_and_not_selector() {
        assert_selector_markdown_standard("*", "*", (
            BorrowLangString { language: "html", value: "<element>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (0, 0, 0)"
        ));
        assert_selector_markdown_standard("#s12:not(foo)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :not>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 0, 1)"
        ));
    }

    #[test]
    fn where_specificity() {
        assert_selector_markdown_standard("#s12:where(foo)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :where>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 0, 0)"
        ));
        assert_selector_markdown_standard("#s12:where(foo > foo, .bar > baz)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :where>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 0, 0)"
        ));
    }

    #[test]
    fn has_not_is_specificity() {
        assert_selector_markdown_standard("#s12:not(foo)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :not>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 0, 1)"
        ));
        assert_selector_markdown_standard("#s12:not(foo > foo)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :not>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 0, 2)"
        ));
        assert_selector_markdown_standard("#s12:not(foo > foo, .bar > baz)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :not>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 1, 1)"
        ));

        assert_selector_markdown_standard("#s12:has(foo)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :has>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 0, 1)"
        ));
        assert_selector_markdown_standard("#s12:has(foo > foo)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :has>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 0, 2)"
        ));
        assert_selector_markdown_standard("#s12:has(foo > foo, .bar > baz)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :has>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 1, 1)"
        ));

        assert_selector_markdown_standard("#s12:is(foo)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :is>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 0, 1)"
        ));
        assert_selector_markdown_standard("#s12:is(foo > foo)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :is>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 0, 2)"
        ));
        assert_selector_markdown_standard("#s12:is(foo > foo, .bar > baz)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :is>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 1, 1)"
        ));

        assert_selector_markdown_standard("#s12:lang(en, fr)", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :lang>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 1, 0)"
        ));

        assert_selector_markdown_standard("#s12:is(foo > foo, :not(.bar > baz, :has(.bar > .baz)))", "#s12", (
            BorrowLangString { language: "html", value: "<element id=\"s12\" :is>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 2, 0)"
        ));
    }

    #[test]
    fn nthchild_nthlastchild_specificity() {
        assert_selector_markdown_standard("#foo:nth-child(2)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" :nth-child>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 1, 0)"
        ));

        assert_selector_markdown_standard("#foo:nth-child(even)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" :nth-child>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 1, 0)"
        ));

        assert_selector_markdown_standard("#foo:nth-child(-n + 2)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" :nth-child>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 1, 0)"
        ));

        assert_selector_markdown_standard("#foo:nth-child(n of.li)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" :nth-child>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 2, 0)"
        ));

        assert_selector_markdown_standard("#foo:nth-child(n of.li,.li.li)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" :nth-child>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 3, 0)"
        ));

        assert_selector_markdown_standard("#foo:nth-child(n of.li, .li.li)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" :nth-child>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 3, 0)"
        ));

        assert_selector_markdown_standard("#foo:nth-child(n of li)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" :nth-child>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 1, 1)"
        ));

        assert_selector_markdown_standard("#foo:nth-child(-n+3 of li.important)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" :nth-child>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 2, 1)"
        ));

        assert_selector_markdown_standard("#foo:nth-child(-n+3 of li.important, .class1.class2.class3)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" :nth-child>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 4, 0)"
        ));

        assert_selector_markdown_standard("#foo:nth-last-child(-n+3 of li, .important)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" :nth-last-child>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 2, 0)"
        ));
    }

    #[test]
    fn host_context_specificity() {
        assert_selector_markdown_standard("#foo:host(.foo)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" :host>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 2, 0)"
        ));

        assert_selector_markdown_standard("#foo:host-context(foo)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" :host-context>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 1, 1)"
        ));
    }

    #[test]
    fn slotted_specificity() {
        assert_selector_markdown_standard("#foo::slotted(foo)", "#foo", (
            BorrowLangString { language: "html", value: "<element id=\"foo\" ::slotted>" },
            "[Selector Specificity](https://developer.mozilla.org/docs/Web/CSS/Specificity): (1, 0, 2)"
        ));
    }
}