use ego_tree::{NodeId, NodeRef, Tree};
use std::{any::Any, collections::{HashMap, VecDeque}, fmt::Debug, ops::Range};

use crate::parser::css_node_types::*;

use super::{css_error::ParseError, css_parser::Parser};

pub trait ITextProvider: std::fmt::Debug {
    fn get_text(&self, offset: usize, length: usize) -> &str;
}

pub trait DataT: Any + std::fmt::Debug + Sync + Send {
    fn noop(&self)
    where
        Self: Sized;
}

#[derive(Debug)]
pub struct CssNode {
    pub offset: usize,
    pub length: usize,
    data: HashMap<String, Box<dyn DataT>>,
    //pub text_provider: Option<Box<dyn ITextProvider + 'a>>,
    pub issues: Vec<Marker>,
    pub node_type: CssNodeType,
}

impl CssNode {
    pub fn new(offset: usize, length: usize, node_type: CssNodeType) -> Self {
        return CssNode {
            offset,
            length,
            data: HashMap::new(),
            //text_provider: None,
            issues: Vec::new(),
            node_type,
        };
    }

    pub fn end(&self) -> usize {
        return self.offset + self.length;
    }

    pub fn collect_issues(&self, results: &mut Vec<Marker>) {
        results.append(&mut self.issues.clone());
    }

    pub fn add_issue(&mut self, issue: Marker) {
        self.issues.push(issue);
    }
    pub fn has_issue(&self, error: ParseError) -> bool {
        return self.issues.iter().any(|i| i.error == error);
    }

    pub fn update_offset_and_length(&mut self, node: &CssNode) {
        if node.offset < self.offset || self.offset == usize::MAX {
            self.offset = node.offset;
        }
        if node.end() > self.end() || self.length == usize::MAX {
            self.length = node.end() - self.offset;
        }
    }

    pub fn encloses(&self, candidate: &CssNode) -> bool {
        return self.offset <= candidate.offset
            && self.offset + self.length >= candidate.offset + candidate.length;
    }

    pub fn set_data<T>(&mut self, key: String, value: Box<T>)
    where
        T: DataT,
    {
        self.data.insert(key, value);
    }

    pub fn get_data(&self, key: &str) -> Option<&Box<dyn DataT>> {
        return self.data.get(key);
    }

    pub fn is_erroneous(&self) -> bool {
        return !self.issues.is_empty();
    }
}

impl PartialEq for CssNode {
    fn eq(&self, other: &Self) -> bool {
        return self.offset == other.offset
            && self.length == other.length
            && self.issues == other.issues
            && self.node_type == other.node_type;
    }
}

#[derive(Debug, PartialEq)]
pub struct Rule {
    pub id: String,
    pub message: String,
}

impl Rule {
    pub fn new(id: &str, message: &str) -> Self {
        return Rule {
            id: id.to_string(),
            message: message.to_string(),
        };
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Level {
    Ignore,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Marker {
    //pub node: NodeData,
    pub error: ParseError,
    pub level: Level,
    pub message: String,
    pub offset: usize,
    pub length: usize,
}

pub trait IVisitor {
    fn visit_node(&mut self, node: NodeRef<CssNode>) -> bool;
}

pub struct ParseErrorCollector {
    pub entries: Vec<Marker>,
}

impl ParseErrorCollector {
    pub fn new() -> Self {
        return ParseErrorCollector {
            entries: Vec::new(),
        };
    }

    // pub fn entries_from_node(node: CssNode) -> Vec<Marker> {
    // 	let mut visitor = ParseErrorCollector::new();
    // 	node.accept_visitor(visitor);
    // 	return visitor.entries;
    // }
}

impl IVisitor for ParseErrorCollector {
    fn visit_node(&mut self, node: NodeRef<CssNode>) -> bool {
        if node.value().is_erroneous() {
            node.value().collect_issues(&mut self.entries);
        }
        return true;
    }
}

// ================
// NodeRef<CssNode>
// ================

pub trait NodeRefExt<'a> {
    fn find_first_child_before_offset(self, offset: usize) -> Option<NodeRef<'a, CssNode>>;
    fn find_child_at_offset(self, offset: usize, go_deep: bool) -> Option<NodeRef<'a, CssNode>>;
    fn get_node_path(self, offset: usize) -> Vec<NodeId>;
    fn get_node_at_offset(self, offset: usize,) -> Option<NodeRef<'a, CssNode>>;
}

impl<'a> NodeRefExt<'a> for NodeRef<'a, CssNode> {
    fn find_first_child_before_offset(self, offset: usize) -> Option<NodeRef<'a, CssNode>> {
        if !self.has_children() {
            return None;
        }
        return self.children().rev().find(|ch| ch.value().offset <= offset);
    }

    fn find_child_at_offset(self, offset: usize, go_deep: bool) -> Option<NodeRef<'a, CssNode>> {
        let current = self.find_first_child_before_offset(offset)?;
        if current.value().end() < offset {
            return None;
        }
        if !go_deep {
            return Some(current);
        }
        return current.find_child_at_offset(offset, true).or(Some(current));
    }

    fn get_node_path(self, offset: usize) -> Vec<NodeId> {
        let mut candidate = self.get_node_at_offset(offset);
        let mut path = VecDeque::new();
        while let Some(cand) = candidate {
            path.push_front(cand.id());
            candidate = cand.parent();
        }
        return path.into();
    }

    fn get_node_at_offset(self, offset: usize,) -> Option<NodeRef<'a, CssNode>> {
        let node_val = self.value();
        let candidate: Option<(NodeId, usize)> = None;
        if offset < node_val.offset || offset > node_val.end() {
            return None;
        }

        // Find the shortest node at the position
        accept_mut(self, move |node: NodeRef<CssNode>| {
            if node_val.offset == usize::MAX && node_val.length == usize::MAX {
                return true;
            }
            if node_val.offset <= offset && node_val.end() >= offset {
                match candidate {
                    None => candidate = Some((node.id(), node.value().length)),
                    Some(cand) => {
                        if node_val.length <= cand.1 {
                            candidate = Some((node.id(), node.value().length));
                        }
                    }
                }
                return true;
            }
            return false;
        });
        if let Some(cand) = candidate {
            return self.tree().get(cand.0);
        }
        return None;
    }
}

// pub fn get_node_at_offset<'a>(
//     node: NodeRef<'a, CssNode>,
//     offset: usize,
// ) -> Option<NodeRef<'a, CssNode>> {
//     let node_val = node.value();
//     let candidate: Option<(NodeId, usize)> = None;
//     if offset < node_val.offset || offset > node_val.end() {
//         return None;
//     }

//     // Find the shortest node at the position
//     todo!();
//     // accept_mut(node, move |node: NodeRef<CssNode>| {
//     //     if node_val.offset == usize::MAX && node_val.length == usize::MAX {
//     //         return true;
//     //     }
//     //     if node_val.offset <= offset && node_val.end() >= offset {
//     //         match candidate {
//     //             None => candidate = Some((node.id(), node.value().length)),
//     //             Some(cand) => {
//     //                 if node_val.length <= cand.1 {
//     //                     candidate = Some((node.id(), node.value().length));
//     //                 }
//     //             }
//     //         }
//     //         return true;
//     //     }
//     //     return false;
//     // });
//     if let Some(cand) = candidate {
//         return node.tree().get(cand.0);
//     }
//     return None;
// }



pub fn get_parent_declaration(node: NodeRef<CssNode>) -> Option<&CssNode> {
    let node_id_dud = node.id();
    let decl = first_ancestor_of_type(
        node,
        CssNodeType::_AbstractDeclaration(AbstractDeclaration {
            colon_position: 0,
            semicolon_position: 0,
            abstract_decl_type: AbstractDeclarationType::Declaration(Declaration {
                property: node_id_dud,
                expr: node_id_dud,
                nested_properties: None,
                declaration_type: DeclarationType::Declaration,
            }),
        }),
    );
    if decl.is_some() && decl.unwrap().value().encloses(node.value()) {
        return decl.map(|d| d.value());
    }
    return None;
}

// pub fn get_first_ancestral_text_provider<'a>(
//     node: NodeRef<CssNode<'a>>,
// ) -> Option<&'a Box<dyn ITextProvider + 'a>> {
//     let mut node = Some(node);
//     while let Some(n) = node {
//         match &n.value().text_provider {
//             None => node = n.parent(),
//             Some(t) => {
//                 return Some(&t);
//             }
//         }
//     }
//     return None;
// }

// pub fn get_text<'a>(node: NodeRef<CssNode>) -> &'a str {
//     if let Some(itp) = get_first_ancestral_text_provider(node) {
//         return itp.get_text(node.value().offset, node.value().length);
//     }
//     return "unknown";
// }

pub fn accept<F>(node: NodeRef<CssNode>, visitor_fun: F)
where
    F: Fn(NodeRef<CssNode>) -> bool,
{
    if visitor_fun(node) {
        for child in node.children() {
            accept(child, &visitor_fun);
        }
    }
}

pub fn accept_mut<F>(node: NodeRef<CssNode>, mut visitor_fun: F) where F: FnMut(NodeRef<CssNode>) -> bool {
    if visitor_fun(node) {
        for child in node.children() {
            accept_mut(child, visitor_fun);
        }
    }
}

pub fn is_erroneous_recursive(node: NodeRef<CssNode>) -> bool {
    return node.value().is_erroneous() || node.children().any(|ch| is_erroneous_recursive(ch));
}

pub fn accept_visitor(node: NodeRef<CssNode>, visitor: &mut impl IVisitor) {
    if visitor.visit_node(node) {
        for child in node.children() {
            accept_visitor(child, visitor);
        }
    }
}

pub fn first_child_before_offset(
    node: NodeRef<CssNode>,
    offset: usize,
) -> Option<NodeRef<CssNode>> {
    if let Some(mut current) = node.children().next() {
        let mut i = node.children().count() - 2;
        loop {
            current = node.children().nth(i).unwrap();
            if current.value().offset <= offset {
                return Some(current);
            }
            if i == 0 {
                break;
            }
            i -= 1;
        }
    }

    return None;
}

pub fn find_child_at_offset(
    node: NodeRef<CssNode>,
    offset: usize,
    go_deep: bool,
) -> Option<NodeRef<CssNode>> {
    let node = first_child_before_offset(node, offset);
    match node {
        None => return None,
        Some(n) => {
            if n.value().end() < offset || !go_deep {
                return None;
            }
            return find_child_at_offset(n, offset, true).or(node);
        }
    }
}

// self excluded, ignores nodelist parents!
pub fn get_real_parent(node: NodeRef<CssNode>) -> Option<NodeRef<CssNode>> {
    let mut result = node.parent();
    while let Some(res) = result {
        if res.value().node_type != CssNodeType::Nodelist {
            break;
        }
        result = res.parent();
    }
    return result;
}

// self excluded
pub fn first_ancestor_of_type(
    node: NodeRef<CssNode>,
    node_type: CssNodeType,
) -> Option<NodeRef<CssNode>> {
    let mut result = node.parent();
    while let Some(res) = result {
        if res.value().node_type.same_node_type(&node_type) {
            break;
        }
        result = res.parent();
    }
    return result;
}

// self excluded
pub fn first_ancestor_of_a_type(
    node: NodeRef<CssNode>,
    node_types: Vec<CssNodeType>,
) -> Option<NodeRef<CssNode>> {
    let mut result = node.parent();
    while let Some(res) = result {
        if node_types
            .iter()
            .any(|nt| nt.same_node_type(&res.value().node_type))
        {
            break;
        }
        result = res.parent();
    }
    return result;
}

pub struct SourceLessCssNodeTree(pub Tree<CssNode>);
impl SourceLessCssNodeTree {
    pub fn new(root: CssNode) -> Self {
        return Self(Tree::new(root));
    }

    pub fn get(&self, node_id: NodeId) -> Option<NodeRef<CssNode>> {
        return self.0.get(node_id);
    }

    pub fn get_text<'a>(&self, node_id: NodeId, source: &'a str) -> &'a str {
        let node = self.0.get(node_id).unwrap();
        let val = node.value();
        let o: usize = val.offset;
        return &source[o..o + val.length];
    }

    pub fn is_attached(&self, node_id: NodeId) -> bool {
        if node_id == self.0.root().id() {
            return true;
        }
        if let Some(p) = self.0.get(node_id).unwrap().parent() {
            return self.is_attached(p.id());
        }
        return false;
    }

    pub fn fancy_string(&self) -> String {
        return Self::fancy_string_internal(self.0.root(), 0).replace("\n\n", "\n");
    }

    fn fancy_string_internal(node: NodeRef<'_, CssNode>, ident: usize) -> String {
        let ident_s = "    ".repeat(ident);
        return "\n".to_owned()
            + &ident_s
            + &format!(
                "{:?}[{:?}]({}-{}) {{",
                node.value().node_type,
                node.id(),
                node.value().offset,
                node.value().length
            )
            + &node
                .children()
                .map(|ch| Self::fancy_string_internal(ch, ident + 1))
                .fold(String::new(), |acc, nex| acc + &nex)
            + "\n"
            + &ident_s
            + "}";
    }
}
impl Default for SourceLessCssNodeTree {
    fn default() -> Self {
        return Self(Tree::new(CssNode {
            offset: usize::MAX,
            length: 0,
            data: HashMap::new(),
            issues: Vec::new(),
            node_type: CssNodeType::ROOT,
        }));
    }
}

pub struct CssNodeTree(pub SourceLessCssNodeTree, pub String);
impl CssNodeTree {
    pub fn new(tree: SourceLessCssNodeTree, source: String) -> Self {
        return Self(tree, source);
    }

    pub fn get(&self, node_id: NodeId) -> Option<NodeRef<CssNode>> {
        return self.0 .0.get(node_id);
    }

    pub fn get_text(&self, node_id: NodeId) -> &str {
        let node = self.0 .0.get(node_id).unwrap();
        let val = node.value();
        let o = val.offset;
        return &self.1[o..o + val.length];
    }

    pub fn matches(&self, node_id: NodeId, s: &str) -> bool {
        return self.get_text(node_id) == s;
    }

    pub fn starts_with(&self, node_id: NodeId, s: &str) -> bool {
        return self.get_text(node_id).starts_with(s);
    }

    pub fn ends_with(&self, node_id: NodeId, s: &str) -> bool {
        return self.get_text(node_id).ends_with(s);
    }

    pub fn fancy_string(&self) -> String {
        return self.0.fancy_string();
    }

    pub fn reparse(&mut self, _replace: Range<usize>, _with_len: usize) {
        // TODO: make incremental
        let res = Parser::new_with_text(std::mem::take(&mut self.1)).into_stylesheet();
        self.0 = res.0;
        self.1 = res.1;
    }
}

impl Default for CssNodeTree {
    fn default() -> Self {
        return CssNodeTree(
            SourceLessCssNodeTree::new(CssNode {
                offset: 0,
                length: 0,
                data: HashMap::new(),
                //text_provider: None,
                issues: Vec::new(),
                node_type: CssNodeType::ROOT,
            }),
            String::new(),
        );
    }
}
