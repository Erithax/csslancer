// use super::syntax::SyntaxKind;
// use super::css_parser2::SyntaxNode;

// /// Macro for declaring simple AST-node
// macro_rules! ast_node {
//     ($ast:ident, $kind:ident) => {
//         #[derive(PartialEq, Eq, Hash)]
//         #[repr(transparent)]
//         struct $ast(SyntaxNode);
//         impl $ast {
//             #[allow(unused)]
//             fn cast(node: SyntaxNode) -> Option<Self> {
//                 if node.kind() == $kind {
//                     Some(Self(node))
//                 } else {
//                     None
//                 }
//             }
//         }
//     };
// }

// pub trait Single {
//     pub fn single(self) -> 
// }

// use super::syntax::SyntaxKind::*;

// ast_node!(Root, ROOT);
// ast_node!(Declarations, DECLARATIONS);

// pub struct BodyDeclaration {
//     declarations: Option<Declarations>,
//     body_declaration_inner: BodyDeclarationInner,
// }

// impl BodyDeclaration {
//     pub fn cast(node: SyntaxNode) -> Option<Self> {
//         if node.kind() == SyntaxKind::BODY_DECLARATION {
//             Some(Self{
//                 declarations: node.children().filter(|ch| ch.kind() == DECLARATIONS).single(),
//                 body_declaration_inner: match node.children().filter(|ch| ch.kind() != Declarations).single().cast(),
//             })
//         } else {
//             None
//         }     
//     }
// }


// pub enum BodyDeclarationInner {
//     RuleSet(RuleSet),
//     CustomPropertySet,
//     IfStatement(IfStatement),
//     ForStatement(ForStatement),
//     EachStatement(EachStatement),
//     WhileStatement,
//     ElseStatement,
//     FunctionDeclaration(FunctionDeclaration),
//     ViewPort,
//     FontFace,
//     NestedProperties,
//     Keyframe(Keyframe),
//     KeyframeSelector,
//     Media,
//     Supports,
//     Layer(Layer),
//     PropertyAtRule(PropertyAtRule),
//     Document,
//     Container,
//     Page,
//     PageBoxMarginBox,
//     MixinContentDeclaration(MixinContentDeclaration),
//     MixinDeclaration(MixinDeclaration),
//     UnknownAtRule(UnknownAtRule),
// }

// ast_node!(CustomPropertySet, CUSTOM_PROPERTY_SET);
// ast_node!(WhileStatement, WHILE_STATEMENT);
// ast_node!(ElseStatement, ELSE_STATEMENT);
// ast_node!(ViewPort, VIEWPORT);
// ast_node!(FontFace, FONT_FACE);
// ast_node!(NestedProperties, NESTED_PROPERTIES);
// ast_node!(KeyframeSelector, KEYFRAME_SELECTOR);
// ast_node!(Media, MEDIA);
// ast_node!(Supports, SUPPORTS);
// ast_node!(Document, DOCUMENT);
// ast_node!(Container, CONTAINER);
// ast_node!(Page, PAGE);
// ast_node!(PageBoxMarginBox, PAGE_BOX_MARGIN_BOX);

// ast_node!(RuleSet, RULE_SET);
// impl RuleSet {
//     pub fn selectors(&self) -> impl Iterator<Item=SyntaxNode> {
//         self.0.children().filter(|ch| ch.kind() == SELECTOR)
//     }
// }

// ast_node!(IfStatement, IF_STATEMENT);
// impl IfStatement {
//     pub fn expression(&self) -> SyntaxNode {
//         self.0.children().filter(|ch| ch.kind() == EXPRESSION).single()
//     }
//     pub fn else_clause_body_decl(&self) -> SyntaxNode {
//         self.0.children().filter(|ch| ch.kind() == BODY_DECLARATION).single()
//     }
// }

// ast_node!(ForStatement, FOR_STATEMENT);
// impl ForStatement {
//     pub fn variable(&self) -> SyntaxNode {
//         self.0.children().filter(|ch| ch.kind() == VARIABLE).single()
//     }
// }

// ast_node!(EachStatement, EACH_STATEMENT);
// impl EachStatement {
//     pub fn variables(&self) -> impl Iterator<Item=SyntaxNode> {
//         self.0.children().filter(|ch| ch.kind() == VARIABLE)
//     }
// }

// ast_node!(FunctionDeclaration, FUNCTION_DECLARATION);
// impl FunctionDeclaration {
//     pub fn identifier(&self) -> SyntaxNode {
//         self.0.children().filter(|ch| ch.kind() == IDENTIFIER).single()
//     }

//     pub fn parameters_nodelist(&self) -> SyntaxNode {
//         self.0.children().filter(|ch| ch.kind() == NODE_LIST).single()
//     }
// }

// ast_node!(Keyframe, KEYFRAME);
// impl Keyframe {
//     pub fn keyword(&self) -> SyntaxNode {
//         self.0.children().filter(|ch| ch.kind() == KEYWORD).single() // TODO: introduce keyword node?
//     }
//     pub fn identifier(&self) -> SyntaxNode {
//         self.0.children().filter(|ch| ch.kind() == IDENTIFIER).single()
//     }
// }

// ast_node!(Layer, LAYER);
// impl Layer {
//     pub fn names(&self) -> SyntaxNode {
//         todo!()
//     }
// }

// ast_node!(PropertyAtRule, PROPERTY_AT_RULE);
// impl PropertyAtRule {
//     pub fn name(&self) -> SyntaxNode {
//         self.0.children().filter(|ch| ch.kind() == todo!()).single()
//     }
// }

// ast_node!(MixinContentDeclaration, MIXIN_CONTENT_DECLARATION);
// impl MixinContentDeclaration {
//     pub fn parameters_nodelist(&self) -> impl Iterator<Item = SyntaxNode> {
//         todo!()
//         self.0.children().filter(|ch| ch == todo!());
//     }
// }

// ast_node!(MixinDeclaration, MIXIN_DECLARATION);
// impl MixinDeclaration {
//     pub fn identifier(&self) -> SyntaxNode {
//         self.0.children().filter(|ch| ch.kind() == IDENTIFIER).single()
//     }
//     pub fn parameters_nodelist(&self) -> impl Iterator<Item = SyntaxNode> {
//         self.0.children().filter(|ch| ch.kind() == todo!())
//     }
//     pub fn less_guard(&self) -> SyntaxNode {
//         self.0.children().filter(|ch| ch.kind() == LESS_GUARD).single()
//     }
// }

// ast_node!(UnknownAtRule, UNKNOWN_AT_RULE);
// impl UnknownAtRule {
//     pub fn at_rule_name_string(&self) -> SyntaxNode {
//         self.0.children().filter(|ch| ch.kind() == todo!()).single()
//     }
// }

