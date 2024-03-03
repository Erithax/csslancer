use crate::parser::css_node_types::*;
use crate::parser::css_nodes::*;

use ego_tree::{NodeId, NodeMut, NodeRef};

pub struct Scope {
    pub offset: usize,
    pub length: usize,
    pub symbols: Vec<Symbol>,
}

impl Scope {
    pub const fn new(offset: usize, length: usize) -> Self {
        return Scope {
            offset,
            length,
            symbols: Vec::new(),
        };
    }

    pub fn add_symbol(&mut self, symbol: Symbol) {
        self.symbols.push(symbol);
    }

    pub fn get_symbol(&self, name: &str, ref_type: ReferenceType) -> Option<&Symbol> {
        return self
            .symbols
            .iter()
            .find(|s| s.name == name && s.ref_type == ref_type);
    }

    pub fn get_symbols(&self) -> &[Symbol] {
        return &self.symbols;
    }
}

pub struct ScopeTree(ego_tree::Tree<Scope>);

impl ScopeTree {
    pub fn new() -> Self {
        return ScopeTree(ego_tree::Tree::new(GLOBAL_SCOPE));
    }

    pub fn add_child(&mut self, parent_id: NodeId, scope: Scope) -> bool {
        if let Some(mut parent_mut) = self.0.get_mut(parent_id) {
            parent_mut.append(scope);
            return true;
        }
        return false;
    }

    pub fn find_scope_id(&self, node_id: NodeId, offset: usize, length: usize) -> Option<NodeId> {
        if let Some(node) = self.0.get(node_id) {
            let scope = node.value();
            if (scope.offset <= offset && scope.offset + scope.length > offset + length)
                || scope.offset == offset && scope.length == length
            {
                return Some(Self::find_in_scope_id(node, offset, length));
            }
        }
        return None;
    }

    // pub fn find_scope_val(&self, node_id: NodeId, offset: usize, length: usize) -> Option<&Scope> {
    //     return self
    //         .find_scope(node_id, offset, length)
    //         .and_then(|nr| Some(nr.value()));
    // }

    // pub fn find_scope_val_mut(&mut self, node_id: NodeId, offset: usize, length: usize) -> Option<&mut Scope> {
    //     return self.find_scope_mut(node_id, offset, length)
    //         .and_then(|mut nm| Some(nm.value()))
    // }

    pub fn find_scope(
        &self,
        node_id: NodeId,
        offset: usize,
        length: usize,
    ) -> Option<NodeRef<Scope>> {
        return self
            .find_scope_id(node_id, offset, length)
            .and_then(|nr| self.0.get(nr));
    }

    pub fn find_scope_mut(
        &mut self,
        node_id: NodeId,
        offset: usize,
        length: usize,
    ) -> Option<NodeMut<Scope>> {
        return self
            .find_scope_id(node_id, offset, length)
            .and_then(|nr| self.0.get_mut(nr));
    }

    pub fn find_in_scope_id(node: NodeRef<Scope>, offset: usize, length: usize) -> NodeId {
        let end = offset + length;
        // find first scope child that has offset larger than end
        let idx = node
            .children()
            .position(|ch| ch.value().offset > end)
            .unwrap_or(node.children().count());

        if idx == 0 {
            // all child scopes have offset larger than end
            return node.id();
        }

        // idx - 1 is last scope child that has offset smaller than end
        let res = node
            .children()
            .nth(idx - 1)
            .expect("internal error: scope child index out of bounds");
        let res_scope = res.value();
        if res_scope.offset <= offset && end <= res_scope.offset + res_scope.length {
            // res_scope encapsulates given argument offset, length
            return Self::find_in_scope_id(res, offset, length);
        }
        return node.id();
    }
}

const GLOBAL_SCOPE: Scope = Scope::new(0, usize::MAX);

pub struct Symbol {
    pub name: String,
    pub value: Option<String>,
    pub ref_type: ReferenceType,
    pub node: CssNode,
}

impl Symbol {
    pub fn new(
        name: String,
        value: Option<String>,
        ref_type: ReferenceType,
        node: CssNode,
    ) -> Self {
        return Symbol {
            name,
            value,
            ref_type,
            node,
        };
    }
}

pub struct ScopeBuilder {
    pub node_id: NodeId,
    pub scope_tree: ScopeTree,
}

impl ScopeBuilder {
    pub fn new(node_id: NodeId) -> Self {
        return ScopeBuilder {
            node_id,
            scope_tree: ScopeTree::new(),
        };
    }

    pub fn add_symbol(
        &mut self,
        name: String,
        value: Option<String>,
        ref_type: ReferenceType,
        css_node: CssNode,
    ) {
        if css_node.offset != usize::MAX {
            if let Some(mut current) =
                self.scope_tree
                    .find_scope_mut(self.node_id, css_node.offset, css_node.length)
            {
                current
                    .value()
                    .add_symbol(Symbol::new(name, value, ref_type, css_node));
            }
        }
    }

    pub fn add_scope(&mut self, css_node: CssNode) -> Option<Scope> {
        if css_node.offset != usize::MAX {
            if let Some(current) =
                self.scope_tree
                    .find_scope(self.node_id, css_node.offset, css_node.length)
            {
                if current.value().offset != css_node.offset
                    || current.value().length != css_node.length
                {
                    let new_scope = Scope::new(css_node.offset, css_node.length);
                    self.scope_tree.add_child(current.id(), new_scope);
                }
            }
        }
        return None;
    }

    pub fn add_symbol_to_child_scope(
        &mut self,
        scope_node: CssNode,
        css_node: CssNode,
        name: String,
        value: Option<String>,
        ref_type: ReferenceType,
    ) {
        if scope_node.offset != usize::MAX {
            if let Some(mut current) = self.add_scope(scope_node) {
                current.add_symbol(Symbol::new(name, value, ref_type, css_node));
                todo!("change add_scope to return a &mut, or NodeId or NodeMut")
            }
        }
    }
}

// impl IVisitor for ScopeBuilder {
//     fn visit_node(&mut self, css_node: NodeRef<CssNode>) -> bool {
//         match css_node.value().node_type {
//             CssNodeType::_BodyDeclaration(b) => {
//                 match b.body_decl_type {
//                     BodyDeclarationType::Keyframe(k) => {
//                         self.add_symbol(get_text(css_node), None, ReferenceType::Keyframe, *css_node.value());
//                     },
//                     BodyDeclarationType::RuleSet(_) => return self.visit_rule_set(css_node),
//                 }
//             }
//             CssNodeType::_AbstractDeclaration(a) => {
//                 match a.abstract_decl_type {
//                     AbstractDeclarationType::Declaration(d) => {
//                         DeclarationType::CustomPropertyDeclaration(_) => return self.visit_custom_property_declaration_node(css_node),
//                         DeclarationType::Declaration(_) => {}
//                     },
//                     AbstractDeclarationType::VariableDeclaration(_) => return self.visit_variable_declaration_node(css_node),
//                 }
//             }
//             CssNodeType::MixinDeclaration(md) => {
//                 self.add_symbol(md.get_name(), None, ReferenceType::Mixin, css_node);
//             },
//             CssNodeType::FunctionDeclaration(f) => {
//                 self.add_scope(f.get_name(), None, ReferenceType::Function, css_node);
//             },
//             CssNodeType::FunctionParameter => return self.visit_function_parameter_node(css_node),
//             CssNodeType::Declarations => {
//                 self.add_scope(css_node);
//             }
//             CssNodeType::For(f) => {
//                 let scope_node = f.declarations;
//                 match f.specific {
//                     BodyDeclarationType::ForStatement(f) => {
//                         self.add_symbol_to_child_scope(scope_node, css_node, f.variable.get_name(), None, ReferenceType::Variable)
//                     },
//                     _ => {}
//                 }
//             },
//             CssNodeType::Each(e) {
//                 let scope_node = e.declarations;
//                 match e.specific {
//                     BodyDeclarationType::EachStatement(e) => {
//                         for variable in e.variables.children
//                         self.add_symbol_to_child_scope(scope_node, css_node, e.v, value, ref_type)
//                     }
//                 }

//             }
//         }
//         return true
//     }
// }
