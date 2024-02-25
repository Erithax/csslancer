
type NodeId = usize;
struct NodeRef<T> {inner: T}

impl<T> NodeRef<T> {
    pub fn id(&self) -> NodeId {
        return 0
    }
    pub fn value(&self) -> T {
        return self.inner;
    }
}

pub struct CssNodeInner {
    pub issues: Vec<String>,
}

pub trait IntoCssNode {
    fn into(self, inner: CssNodeInner) -> CssNodeType;
}

pub enum CssNodeType {
    John(CssNodeInner, John),
    Jane(CssNodeInner, Jane),
    BodyDeclaration(CssNodeInner, BodyDeclaration)
}

impl CssNodeType {
    pub fn inner(&self) -> &CssNodeInner {
        match self {
            Self::John(c, _) |
            Self::Jane(c, _) |
            Self::BodyDeclaration(c, _) => return c
        }
    }
}

pub struct John{}
pub struct Jane{}
pub struct Dolphin{}

pub enum BodyDeclaration {
    Bat(Bat),
    Dolphin(Dolphin)
}

pub struct Bat {
    john: NodeId,
    dolphin: NodeId,
}

impl Bat {

    pub fn new(john: NodeRef<CssNodeType>, dolphin: NodeRef<CssNodeType> ) -> Self {
        //macroni!(john, John);
        //macroni!(dolphin, BodyDeclaration, Dolphin);
        match john.value() {
            CssNodeType::John(..) => {},
            _ => {
                #[cfg(debug_assertions)]
                panic!("no bueno");
                john.value().inner().issues().push("no bueno");
            }
        }
        match dolphin.value() {
            CssNodeType::BodyDeclaration(_, b) => {
                match b {
                    BodyDeclaration::Dolphin(..) => {},
                    _ => {
                        #[cfg(debug_assertions)]
                        panic!("no bueno");
                        dolphin.value().inner().issues().push("no bueno");
                    }
                }
            },
            _ => {
                #[cfg(debug_assertions)]
                panic!("no bueno");
                dolphin.value().inner().issues().push("no bueno");
            }
        }
        return Self {
            john: john.id(),
            dolphin: dolphin.id(),
        }
    }
}
impl IntoCssNode for Bat {
    fn into(self, inner: CssNodeInner) -> CssNodeType {
        return CssNodeType::BodyDeclaration(
            inner,
            BodyDeclaration::Bat(
                self
            )
        )
    }
}