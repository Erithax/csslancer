use ego_tree::{NodeId, NodeRef, Tree};
use std::{
    any::Any,
    collections::{HashMap, VecDeque},
    fmt::Debug, ops::Range,
};

use crate::parser::css_node_types::*;

use super::{css_error::ParseError, css_parser::Parser};

pub trait ITextProvider: std::fmt::Debug {
    fn get_text(&self, offset: usize, length: usize) -> &str;
}

trait DataT: Any + std::fmt::Debug + Sync + Send {
    fn noop(&self)
    where
        Self: Sized;
}

#[derive(Debug)]
pub struct CssNode {
    pub offset: usize,
    pub length: usize,
    pub data: HashMap<String, Box<dyn DataT>>,
    //pub text_provider: Option<Box<dyn ITextProvider + 'a>>,
    pub issues: Vec<Marker>,
    pub node_type: CssNodeType,
}

impl CssNode {
    pub fn new(offset: usize, length: usize, node_type: CssNodeType) -> Self {
        return CssNode {
            offset: offset,
            length: length,
            data: HashMap::new(),
            //text_provider: None,
            issues: Vec::new(),
            node_type: node_type,
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
        return self.issues.len() > 0;
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
pub fn get_node_at_offset<'a>(node: NodeRef<'a, CssNode>, offset: usize) -> Option<NodeRef<'a, CssNode>> {
    let node_val = node.value();
    let mut candidate: Option<(NodeId, usize)> = None;
    if offset < node_val.offset || offset > node_val.end() {
        return None;
    }

    // Find the shortest node at the position
    todo!();
    // accept_mut(node, move |node: NodeRef<CssNode>| {
    //     if node_val.offset == usize::MAX && node_val.length == usize::MAX {
    //         return true;
    //     }
    //     if node_val.offset <= offset && node_val.end() >= offset {
    //         match candidate {
    //             None => candidate = Some((node.id(), node.value().length)),
    //             Some(cand) => {
    //                 if node_val.length <= cand.1 {
    //                     candidate = Some((node.id(), node.value().length));
    //                 }
    //             }
    //         }
    //         return true;
    //     }
    //     return false;
    // });
    if let Some(cand) = candidate {
        return node.tree().get(cand.0);
    }
    return None;
}

pub fn get_node_path<'a>(node: NodeRef<'a, CssNode>, offset: usize) -> Vec<&'a CssNode> {
    let mut candidate = get_node_at_offset(node, offset);
    let mut path: VecDeque<&CssNode> = VecDeque::new();
    loop {
        match candidate {
            None => break,
            Some(cand) => {
                path.push_front(cand.value());
                candidate = cand.parent();
            }
        }
    }
    return path.into();
}

pub fn get_parent_declaration<'a>(node: NodeRef<'a, CssNode>) -> Option<&'a CssNode> {
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


pub fn accept_mut(node: NodeRef<CssNode>, mut visitor_fun: fn(NodeRef<CssNode>) -> bool)
{
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

pub fn first_child_before_offset<'a, 'b>(
    node: NodeRef<'b, CssNode>,
    offset: usize,
) -> Option<NodeRef<'b, CssNode>> {
    if let Some(mut current) = node.children().nth(0) {
        let mut i = node.children().count() - 2;
        while i >= 0 {
            current = node.children().nth(i).unwrap();
            if current.value().offset <= offset {
                return Some(current);
            }
            i -= 1;
        }
    }

    return None;
}

pub fn find_child_at_offset<'a, 'b>(
    node: NodeRef<'b, CssNode>,
    offset: usize,
    go_deep: bool,
) -> Option<NodeRef<'b, CssNode>> {
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
pub fn get_real_parent<'a, 'b>(node: NodeRef<'b, CssNode>) -> Option<NodeRef<'b, CssNode>> {
    let mut result = node.parent();
    loop {
        match result {
            Some(n) => {
                if n.value().node_type != CssNodeType::Nodelist {
                    break;
                }
                result = n.parent();
            }
            None => break,
        }
    }
    return result;
}

// self excluded
pub fn first_ancestor_of_type<'a, 'b>(
    node: NodeRef<'b, CssNode>,
    node_type: CssNodeType,
) -> Option<NodeRef<'b, CssNode>> {
    let mut result = node.parent();
    loop {
        match result {
            Some(n) => {
                if n.value().node_type.same_node_type(&node_type) {
                    break;
                }
                result = n.parent();
            }
            None => break,
        }
    }
    return result;
}

// self excluded
pub fn first_ancestor_of_a_type<'a, 'b>(
    node: NodeRef<'b, CssNode>,
    node_types: Vec<CssNodeType>,
) -> Option<NodeRef<'b, CssNode>> {
    let mut result = node.parent();
    loop {
        match result {
            Some(n) => {
                if node_types
                    .iter()
                    .any(|nt| nt.same_node_type(&n.value().node_type))
                {
                    break;
                }
                result = n.parent();
            }
            None => break,
        }
    }
    return result;
}

pub struct SourceLessCssNodeTree(pub Tree<CssNode>);
impl SourceLessCssNodeTree {
    pub fn new(root: CssNode) -> Self {
        return Self(Tree::new(root));
    }
    pub fn get_text<'a>(&self, node_id: NodeId, source: &'a str) -> &'a str {
        let node = self.0.get(node_id).unwrap();
        let val = node.value();
        let o: usize = val.offset;
        return &source[o..o+val.length]
    }

    pub fn is_attached(&self, node_id: NodeId) -> bool {
        if node_id == self.0.root().id() {
            return true
        }
        if let Some(p) = self.0.get(node_id).unwrap().parent() {
            return self.is_attached(p.id());
        }
        return false
    }

    pub fn fancy_string(&self) -> String {
        return Self::fancy_string_internal(self.0.root());
    }

    fn fancy_string_internal(node: NodeRef<'_, CssNode>) -> String {
        let self_str = format!("{:?}\n", node.value().node_type);
        return self_str + &node.children()
            .into_iter()
            .map(|ch| 
                Self::fancy_string_internal(ch).lines()
                .map(|l| "\t".to_owned() + &l + "\n")
                .fold(String::new(), |acc, nex| acc + &nex)
            )
            .fold(String::new(), |acc, nex| acc + &nex)
    }
}
impl Default for SourceLessCssNodeTree {
    fn default() -> Self {
        return Self(Tree::new(CssNode {
            offset: usize::MAX,
            length: usize::MAX,
            data: HashMap::new(),
            issues: Vec::new(),
            node_type: CssNodeType::ROOT,
        }))
    }

}

pub struct CssNodeTree(pub Tree<CssNode>, pub String);
impl CssNodeTree {
    pub fn new(tree: SourceLessCssNodeTree, source: String) -> Self {
        return Self(tree.0, source);
    }

    // fn get_first_ancestral_text_provider(
    //     &self,
    //     node_id: NodeId,
    // ) -> Option<&Box<dyn ITextProvider>> {
    //     if let Some(node) = self.0.get(node_id) {
    //         return get_first_ancestral_text_provider(node);
    //     }
    //     return None;
    // }

    pub fn get_text(&self, node_id: NodeId) -> &str {
        let node = self.0.get(node_id).unwrap();
        let val = node.value();
        let o = val.offset;
        return &self.1[o..o+val.length]
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
        return Self::fancy_string_internal(self.0.root());
    }

    fn fancy_string_internal(node: NodeRef<'_, CssNode>) -> String {
        let self_str = format!("{:?}\n", node.value().node_type);
        return self_str + &node.children()
            .into_iter()
            .map(|ch| 
                Self::fancy_string_internal(ch).lines()
                .map(|l| "    ".to_owned() + &l)
                .fold(String::new(), |acc, nex| acc + &nex)
            )
            .fold(String::new(), |acc, nex| acc + "\n" + &nex)
    }

    pub fn reparse(&mut self, replace: Range<usize>, with_len: usize) {
        let res = Parser::new_with_text(std::mem::take(&mut self.1)).into_stylesheet();
        self.0 = res.0;
        self.1 = res.1;
    }

    // 	pub fn adopt_child(&mut self, mut child: &mut CssNode, index: Option<usize>) {
    // 		// remove child from previous parent
    // 		if let Some(mut par) = child.parent {
    // 			par.children.retain(|n: &Box<CssNode>| **n != *child);
    // 		}

    // 		child.parent = Some(Box::new(*self));
    // 		match index {
    // 			None => self.children.push(Box::new(*child)),
    // 			Some(i) => self.children[i] = Box::new(*child),
    // 		}
    // 	}

    // 	pub fn attach_parent(&mut self, mut parent: CssNode, index: Option<usize>) {
    // 		parent.adopt_child(self, index);
    // 	}`

    // pub fn is_erroneous_recursive(&self) -> bool {
    // 	return self.issues.len() > 0 && self.children.iter().any(|c| c.is_erroneous(recursive));
    // }

    // pub fn set_node() -> bool {
    // 	todo!()
    // }

    // pub fn add_child(&mut self, mut node: &mut CssNode) {
    // 	node.attach_parent(*self, None);
    // 	self.update_offset_and_length(&*node);
    // }

    // pub fn has_children(&self) -> bool {
    // 	return self.children.len() > 0;
    // }

    // pub fn get_children(&self) -> &Vec<Box<CssNode>> {
    // 	return &self.children;
    // }

    // pub fn get_child(&self, index: usize) -> Option<&Box<CssNode>> {
    // 	return self.children.get(index);
    // }

    // pub fn add_children(&mut self, mut nodes: Vec<CssNode>) {
    // 	self.children.append(&mut nodes.into_iter().map(|n| Box::new(n)).collect());
    //
}

impl Default for CssNodeTree {
    fn default() -> Self {
        return CssNodeTree(Tree::new(CssNode {
            offset: 0,
            length: 0,
            data: HashMap::new(),
            //text_provider: None,
            issues: Vec::new(),
            node_type: CssNodeType::ROOT,
        }), String::new());
    }
}
