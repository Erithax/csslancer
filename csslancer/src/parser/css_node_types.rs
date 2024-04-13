#![allow(clippy::single_match)]

use crate::parser::css_nodes::*;
use ego_tree::{NodeId, NodeRef};

#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceType {
    Mixin,
    Rule,
    Variable,
    Function,
    Keyframe,
    Unknown,
    Module,
    Forward,
    ForwardVisibility,
    Property,
}

/// <summary>
/// Nodes for the css 2.1 specification. See for reference:
/// http://www.w3.org/TR/CSS21/grammar.html#grammar
/// </summary>
#[derive(Debug, PartialEq, Default)]
pub enum CssNodeType {
    #[default]
    ROOT,
    _BodyDeclaration(BodyDeclaration),
    _AbstractDeclaration(AbstractDeclaration),
    _Invocation(Invocation),

    LessGuard,
    Variable,
    CustomPropertyValue,
    Medialist,

    Nodelist,
    Undefined,

    Identifier(Identifier),
    Stylesheet,
    Selector,
    SimpleSelector,
    SelectorInterpolation,
    SelectorCombinator,
    SelectorCombinatorParent,
    SelectorCombinatorSibling,
    SelectorCombinatorAllSiblings,
    SelectorCombinatorShadowPiercingDescendant,
    ClassSelector,
    IdentifierSelector,
    ElementNameSelector,
    PseudoSelector,
    AttributeSelector(AttributeSelector),
    Declarations,
    Property(Property),
    Expression,
    BinaryExpression(BinaryExpression),
    Term(Term),
    Operator,
    Value,
    StringLiteral,
    URILiteral,
    EscapedValue,
    NumericValue,
    HexColorValue,
    RatioValue,
    MixinReference(MixinReference),
    VariableName,
    VariableDeclaration(VariableDeclaration),
    Prio,
    Interpolation,
    ExtendsReference,
    SelectorPlaceholder,
    Debug,
    MixinContentReference,
    Import,
    Namespace,
    ReturnStatement,
    MediaQuery,
    MediaCondition,
    MediaFeature,
    FunctionParameter,
    FunctionArgument(FunctionArgument),
    AtApplyRule,
    ListEntry,
    SupportsCondition(SupportsCondition),
    NamespacePrefix,
    GridLine,
    Plugin,
    Use,
    ModuleConfiguration,
    Forward,
    ForwardVisibility,
    Module,
    UnicodeRange(UnicodeRange),
    LayerNameList,
    LayerName,
}

impl CssNodeType {
    pub fn same_node_type(&self, other: &Self) -> bool {
        use CssNodeType::*;

        fn variant_eq<T>(a: &T, b: &T) -> bool {
            std::mem::discriminant(a) == std::mem::discriminant(b)
        }

        match (self, other) {
            (_BodyDeclaration(a), _BodyDeclaration(b)) => {
                return variant_eq(&a.body_decl_type, &b.body_decl_type)
            }
            (_AbstractDeclaration(a), _AbstractDeclaration(b)) => {
                match (&a.abstract_decl_type, &b.abstract_decl_type) {
                    (
                        AbstractDeclarationType::Declaration(a),
                        AbstractDeclarationType::Declaration(b),
                    ) => return variant_eq(&a.declaration_type, &b.declaration_type),
                    _ => return variant_eq(&a, &b),
                }
            }
            (_Invocation(a), _Invocation(b)) => {
                return variant_eq(&a.invocation_type, &b.invocation_type)
            }
            _ => {}
        }
        return variant_eq(self, other);
    }
}

#[derive(Debug, PartialEq)]
pub struct UnicodeRange {
    pub range_start: NodeId, // any node type
    pub range_end: NodeId,   // any node type
}

#[derive(Debug, PartialEq)]
pub struct Identifier {
    pub reference_types: Option<Vec<ReferenceType>>,
    pub is_custom_property: bool,
}
impl Identifier {
    pub fn contains_interpolation(&self, self_node: NodeRef<CssNode>) -> bool {
        return self_node.has_children();
    }
}

#[derive(Debug, PartialEq)]
pub struct AtApplyRule {
    pub identifier: NodeId, // Identifier
}

// ===============
// BodyDeclaration
// ===============
#[derive(Debug, PartialEq)]
pub struct BodyDeclaration {
    pub declarations: Option<NodeId>,
    pub body_decl_type: BodyDeclarationType,
}
impl CssNodeType {
    pub fn unchecked_body_decl(&mut self) -> &mut BodyDeclaration {
        match self {
            CssNodeType::_BodyDeclaration(ref mut b) => return b,
            _ => {}
        }
        unreachable!()
    }
}

#[derive(Debug, PartialEq)]
pub enum BodyDeclarationType {
    RuleSet(RuleSet),
    CustomPropertySet,
    IfStatement(IfStatement),
    ForStatement(ForStatement),
    EachStatement(EachStatement),
    WhileStatement,
    ElseStatement,
    FunctionDeclaration(FunctionDeclaration),
    ViewPort,
    FontFace,
    NestedProperties,
    Keyframe(Keyframe),
    KeyframeSelector,
    Media,
    Supports,
    Layer(Layer),
    PropertyAtRule(PropertyAtRule),
    Document,
    Container,
    Page,
    PageBoxMarginBox,
    MixinContentDeclaration(MixinContentDeclaration),
    MixinDeclaration(MixinDeclaration),
    UnknownAtRule(UnknownAtRule),
}

#[derive(Debug, PartialEq)]
pub struct RuleSet {
    pub selectors: NodeId,
}
impl CssNodeType {
    pub fn unchecked_rule_set(&mut self) -> &mut RuleSet {
        match self {
            CssNodeType::_BodyDeclaration(ref mut b) => match b.body_decl_type {
                BodyDeclarationType::RuleSet(ref mut r) => return r,
                _ => {}
            },
            _ => {}
        }
        unreachable!()
    }

    pub fn unchecked_rule_set_ref(&self) -> &RuleSet {
        match self {
            CssNodeType::_BodyDeclaration(ref b) => match b.body_decl_type {
                BodyDeclarationType::RuleSet(ref r) => return r,
                _ => {}
            },
            _ => {}
        }
        unreachable!()
    }
}
// impl RuleSet {
//     pub fn new(nodelist: NodeRef<CssNode>) -> Option<Self> {
//         match nodelist.value().node_type {
//             CssNodeType::Nodelist => Some(RuleSet {
//                 selectors: nodelist.id(),
//             }),
//             _ => {None}
//         }
//     }

// 	pub fn get_selectors(&self) -> NodeId {
// 		return self.selectors;
// 	}
// }

#[derive(Debug, PartialEq)]
pub struct IfStatement {
    pub expression: NodeId,
    pub else_clause: NodeId,
}

#[derive(Debug, PartialEq)]
pub struct ForStatement {
    pub variable: NodeId,
}

#[derive(Debug, PartialEq)]
pub struct EachStatement {
    pub variables: NodeId,
}

#[derive(Debug, PartialEq)]
pub struct FunctionDeclaration {
    pub identifier: NodeId,
    pub parameters: NodeId,
}

#[derive(Debug, PartialEq)]
pub struct Keyframe {
    pub keyword: NodeId,    // keyword
    pub identifier: NodeId, // Identifier
}
impl CssNodeType {
    pub fn unchecked_inner_keyword(&mut self) -> &mut Keyframe {
        match self {
            CssNodeType::_BodyDeclaration(ref mut b) => match b.body_decl_type {
                BodyDeclarationType::Keyframe(ref mut k) => return k,
                _ => {}
            },
            _ => {}
        }
        unreachable!()
    }
}

#[derive(Debug, PartialEq)]
pub struct Layer {
    pub names: NodeId, // any
}
impl CssNodeType {
    pub fn unchecked_layer(&mut self) -> &mut Layer {
        match self {
            CssNodeType::_BodyDeclaration(b) => match b.body_decl_type {
                BodyDeclarationType::Layer(ref mut l) => return l,
                _ => {}
            },
            _ => {}
        }
        unreachable!()
    }
}

#[derive(Debug, PartialEq)]
pub struct PropertyAtRule {
    pub name: NodeId, // Identifier
}
impl CssNodeType {
    pub fn unchecked_property_at_rule(&mut self) -> &mut PropertyAtRule {
        match self {
            CssNodeType::_BodyDeclaration(b) => match b.body_decl_type {
                BodyDeclarationType::PropertyAtRule(ref mut par) => return par,
                _ => {}
            },
            _ => {}
        }
        unreachable!()
    }
}

#[derive(Debug, PartialEq)]
pub struct MixinContentDeclaration {
    pub parameters: NodeId, // Nodelist
}

#[derive(Debug, PartialEq)]
pub struct MixinDeclaration {
    pub identifier: NodeId, // Identifier
    pub parameters: NodeId, // Nodelist
    pub guard: NodeId,      // LessGuard
}

impl CssNodeType {
    pub fn unchecked_mixin_declaration(&mut self) -> &mut MixinDeclaration {
        if let CssNodeType::_BodyDeclaration(BodyDeclaration { 
            declarations: _, 
            body_decl_type: BodyDeclarationType::MixinDeclaration(m)}) = self {
            return m
        }
        unreachable!();
    }
    pub fn unchecked_mixin_declaration_ref(&self) -> &MixinDeclaration {
        if let CssNodeType::_BodyDeclaration(BodyDeclaration { 
            declarations: _, 
            body_decl_type: BodyDeclarationType::MixinDeclaration(m)}) = self {
            return m
        }
        unreachable!();
    }
}

#[derive(Debug, PartialEq)]
pub struct UnknownAtRule {
    pub at_rule_name: String,
}

// ===============
// AbstractDeclaration
// ===============

#[derive(Debug, PartialEq)]
pub struct AbstractDeclaration {
    pub colon_position: usize,
    pub semicolon_position: usize,
    pub abstract_decl_type: AbstractDeclarationType,
}
impl CssNodeType {
    pub fn unchecked_abst_decl_inner(&mut self) -> &mut AbstractDeclaration {
        match self {
            CssNodeType::_AbstractDeclaration(a) => {
                return a;
            }
            _ => {}
        }
        unreachable!();
    }
}

#[derive(Debug, PartialEq)]
pub enum AbstractDeclarationType {
    Declaration(Declaration),
    VariableDeclaration(VariableDeclaration),
}

#[derive(Debug, PartialEq)]
pub struct Declaration {
    pub property: NodeId,                  // Property
    pub expr: NodeId,                      // Expression
    pub nested_properties: Option<NodeId>, // NestedProperties
    pub declaration_type: DeclarationType,
}
impl CssNodeType {
    pub fn unchecked_abst_decl_decl_decl_inner(&mut self) -> &mut Declaration {
        match self {
            CssNodeType::_AbstractDeclaration(a) => match a.abstract_decl_type {
                AbstractDeclarationType::Declaration(ref mut d) => return d,
                _ => {}
            },
            _ => {}
        }
        unreachable!();
    }

    pub fn unchecked_abst_decl_decl_decl_inner_ref(&self) -> &Declaration {
        match self {
            CssNodeType::_AbstractDeclaration(a) => match a.abstract_decl_type {
                AbstractDeclarationType::Declaration(ref d) => return d,
                _ => {}
            },
            _ => {}
        }
        unreachable!();
    }
}

#[derive(Debug, PartialEq)]
pub enum DeclarationType {
    Declaration,
    CustomPropertyDeclaration(CustomPropertyDeclaration),
}

#[derive(Debug, PartialEq)]
pub struct VariableDeclaration {
    pub variable: NodeId, // Variable
    pub value: NodeId,    // any
    pub needs_semicolon: bool,
}

#[derive(Debug, PartialEq)]
pub struct CustomPropertyDeclaration {
    pub property_set: NodeId, // CustomPropertySet
}

impl CssNodeType {
    pub fn unchecked_abst_decl_decl_custom_prop_decl_inner(
        &mut self,
    ) -> &mut CustomPropertyDeclaration {
        match self {
            CssNodeType::_AbstractDeclaration(a) => match a.abstract_decl_type {
                AbstractDeclarationType::Declaration(ref mut d) => match d.declaration_type {
                    DeclarationType::CustomPropertyDeclaration(ref mut c) => {
                        return c;
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
        unreachable!();
    }
}

// ===================================

#[derive(Debug, PartialEq)]
pub struct Property {
    pub identifier: NodeId,
}
impl Property {
    pub fn is_custom_property(&self, tree: &CssNodeTree) -> bool {
        match &tree.0.get(self.identifier).unwrap().value().node_type {
            CssNodeType::Identifier(i) => {
                return i.is_custom_property;
            }
            _ => panic!("no bueno"),
        }
    }
}
impl CssNodeType {
    pub fn unchecked_inner_property(&mut self) -> &mut Property {
        match self {
            CssNodeType::Property(ref mut p) => return p,
            _ => {
                unreachable!()
            }
        }
    }
}

// ========================
// INVOCATION
// ========================

#[derive(Debug, PartialEq)]
pub struct Invocation {
    pub arguments: NodeId, // Nodelist
    pub invocation_type: InvocationType,
}

#[derive(Debug, PartialEq)]
pub enum InvocationType {
    Invocation,
    Function(Function),
}

#[derive(Debug, PartialEq)]
pub struct Function {
    pub identifier: NodeId, // Identifier
}
impl CssNodeType {
    pub fn unchecked_function(&mut self) -> &mut Function {
        match self {
            CssNodeType::_Invocation(i) => match i.invocation_type {
                InvocationType::Function(ref mut f) => return f,
                _ => {}
            },
            _ => {}
        }
        unreachable!()
    }
}

// -----------------------

#[derive(Debug, PartialEq)]
pub struct FunctionParameter {
    pub identifier: NodeId,    // any TODO: SHOULD BE IDENTIFIER?
    pub default_value: NodeId, // any
}

#[derive(Debug, PartialEq)]
pub struct FunctionArgument {
    pub identifier: Option<NodeId>, // any, TODO:> SHOUDL BE IDENTIFIER?
    pub value: NodeId,              // any
}
impl CssNodeType {
    pub fn unchecked_function_argument(&mut self) -> &mut FunctionArgument {
        match self {
            CssNodeType::FunctionArgument(ref mut fa) => return fa,
            _ => {}
        }
        unreachable!()
    }
}

#[derive(Debug, PartialEq)]
pub struct Use {
    pub identifier: NodeId, // Identifier
    pub parameters: NodeId, // Nodelist
}

#[derive(Debug, PartialEq)]
pub struct ModuleConfiguration {
    pub identifier: NodeId, // any
    pub value: NodeId,      // any
}

#[derive(Debug, PartialEq)]
pub struct Forward {
    pub identifier: NodeId, // any
    pub members: NodeId,    // Nodelist,
    pub parameters: NodeId, // Nodelist
}

#[derive(Debug, PartialEq)]
pub struct ForwardVisibility {
    pub identifier: NodeId, // any
}

#[derive(Debug, PartialEq)]
pub struct SupportsCondition {
    pub lef_parent: usize,
    pub rig_parent: usize,
}
impl CssNodeType {
    pub fn unchecked_supports_condition(&mut self) -> &mut SupportsCondition {
        match self {
            CssNodeType::SupportsCondition(c) => return c,
            _ => {}
        }
        unreachable!()
    }
}

#[derive(Debug, PartialEq)]
pub struct BinaryExpression {
    pub left: NodeId,     // any
    pub right: NodeId,    // any
    pub operator: NodeId, // any
}
impl CssNodeType {
    pub fn unchecked_binary_expr(&mut self) -> &mut BinaryExpression {
        match self {
            CssNodeType::BinaryExpression(ref mut b) => return b,
            _ => {}
        }
        unreachable!()
    }
}

#[derive(Debug, PartialEq)]
pub struct Term {
    pub operator: Option<NodeId>, // any
    pub expression: NodeId,       // any
}
impl CssNodeType {
    pub fn unchecked_term(&mut self) -> &mut Term {
        match self {
            CssNodeType::Term(ref mut t) => return t,
            _ => {}
        }
        unreachable!()
    }
}

#[derive(Debug, PartialEq)]
pub struct AttributeSelector {
    pub namespace_prefix: Option<NodeId>, // any
    pub identifier: NodeId,               // Identifier
    pub operator: NodeId,                 // Operator
    pub value: Option<NodeId>,            // BinaryExpression
}
impl CssNodeType {
    pub fn unchecked_attribute_selector(&mut self) -> &mut AttributeSelector {
        match self {
            CssNodeType::AttributeSelector(ref mut a) => return a,
            _ => {}
        }
        unreachable!()
    }
}

#[derive(Debug, PartialEq)]
pub struct NumericValue {}

#[derive(Debug, PartialEq)]
pub struct NumericValueParsed {
    value: String,
    unit: Option<String>,
}

impl NumericValue {
    // pub fn get_value(self_node: NodeRef<CssNode>) -> NumericValueParsed {
    //     let raw = get_text(self_node);
    //     let mut unit_idx = 0;
    //     for char in raw.chars() {
    //         if !char.is_ascii_digit() {
    //             break;
    //         }
    //         unit_idx += 1;
    //     }
    //     let (val, unit) = raw.split_at(unit_idx);
    //     return NumericValueParsed {
    //         value: val.to_string(),
    //         unit: (!unit.is_empty()).then(|| unit.to_string()),
    //     };
    // }
}

#[derive(Debug, PartialEq)]
pub struct Variable {
    pub module: NodeId, // Module
}

#[derive(Debug, PartialEq)]
pub struct ExtendsReference {
    pub selectors: NodeId, // Nodelist
}

#[derive(Debug, PartialEq)]
pub struct MixinContentReference {
    pub arguments: NodeId, // Nodelist
}

#[derive(Debug, PartialEq)]
pub struct MixinReference {
    pub namespaces: NodeId,      // Nodelist
    pub identifier: NodeId,      // Identifier
    pub arguments: NodeId,       // Nodelist
    pub content: Option<NodeId>, // MixinContentDeclaration
}

#[derive(Debug, PartialEq)]
pub struct ListEntry {
    pub key: NodeId,   // any
    pub value: NodeId, // any
}

#[derive(Debug, PartialEq)]
pub struct LessGuard {
    pub conditions: NodeId, // Nodelist
}

#[derive(Debug, PartialEq)]
pub struct GuardCondition {
    pub variable: NodeId, // any
    pub is_negated: bool,
    pub is_equals: bool,
    pub is_greater: bool,
    pub is_equals_greater: bool,
    pub is_less: bool,
    pub is_equals_less: bool,
}

#[derive(Debug, PartialEq)]
pub struct Module {
    pub identifier: NodeId, // Identifier
}
