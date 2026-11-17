// ------------------------------------------------------------
// RUSTAVA — Arbre de syntaxe abstraite
// Le mariage : la FORME de Java (classes, accolades, ;)
//              + la MÉMOIRE de Rust (let immuable, var mutable)
// ------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Arr(Box<Type>),
    Class(String),
    Interface(String),
    Void,
}

impl Type {
    pub fn affiche(&self) -> String {
        match self {
            Type::Int => "int".into(),
            Type::Float => "float".into(),
            Type::Bool => "bool".into(),
            Type::Str => "string".into(),
            Type::Arr(t) => format!("{}[]", t.affiche()),
            Type::Class(c) => c.clone(),
            Type::Interface(i) => i.clone(),
            Type::Void => "void".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Clone, Debug)]
pub enum Item {
    Class(ClassDecl),
    Interface(InterfaceDecl),
    Fn(FnDecl),
}

#[derive(Clone, Debug)]
pub struct ClassDecl {
    pub name: String,
    pub interfaces: Vec<String>,
    pub fields: Vec<FieldDecl>,
    pub methods: Vec<MethodDecl>,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct FieldDecl {
    pub mutable: bool,
    pub name: String,
    pub ty: Type,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct MethodDecl {
    pub name: String, // == nom de la classe => constructeur
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub body: Block,
    pub line: usize,
}

impl MethodDecl {
    pub fn est_constructeur(&self, classe: &str) -> bool {
        self.name == classe
    }
}

#[derive(Clone, Debug)]
pub struct InterfaceDecl {
    pub name: String,
    pub methods: Vec<MethSig>,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct MethSig {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub body: Block,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    // let nom = expr;   /  var nom (: type)? = expr;
    Decl {
        mutable: bool,
        name: String,
        ty: Option<Type>,
        init: Expr,
        line: usize,
    },
    // cible = valeur ;  ou  cible += valeur (op == None => affectation simple)
    Assign {
        target: Expr,
        op: Option<BinOp>,
        value: Expr,
        line: usize,
    },
    ExprStmt(Expr, usize),
    If {
        arms: Vec<(Expr, Block)>, // cond, bloc
        else_blk: Option<Block>,
        line: usize,
    },
    While {
        cond: Expr,
        body: Block,
        line: usize,
    },
    For {
        var: String,
        iter: Iter,
        body: Block,
        line: usize,
    },
    Match {
        subject: Expr,
        arms: Vec<MatchArm>,
        line: usize,
    },
    Try {
        body: Block,
        catch_var: Option<String>,
        catch_blk: Block,
        line: usize,
    },
    Throw(Expr, usize),
    Return(Option<Expr>, usize),
    Spawn(Expr, usize), // forcément un appel (vérifié par le parser)
    Block(Block, usize),
}

#[derive(Clone, Debug)]
pub enum Iter {
    Range(Expr, Expr), // a..b (b exclu, comme Rust)
    Items(Expr),       // tableau
}

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub pat: MatchPat,
    pub body: Block, // un seul statement devient un bloc
    pub line: usize,
}

#[derive(Clone, Debug)]
pub enum MatchPat {
    IntLit(i64),
    FloatLit(f64),
    StrLit(String),
    BoolLit(bool),
    Wildcard,
}

#[derive(Clone, Debug)]
pub enum Expr {
    IntLit(i64, usize),
    FloatLit(f64, usize),
    StrLit(String, usize),
    BoolLit(bool, usize),
    Ident(String, usize),
    // afficher(x) — fn top-niveau OU méthode de la même classe (résolu au typage)
    Call {
        callee: String,
        args: Vec<Expr>,
        line: usize,
    },
    MethodCall {
        obj: Box<Expr>,
        name: String,
        args: Vec<Expr>,
        line: usize,
    },
    Field {
        obj: Box<Expr>,
        name: String,
        line: usize,
    },
    Index {
        obj: Box<Expr>,
        idx: Box<Expr>,
        line: usize,
    },
    New {
        class: String,
        args: Vec<Expr>,
        line: usize,
    },
    ArrayLit {
        items: Vec<Expr>,
        line: usize,
    },
    Bin {
        op: BinOp,
        l: Box<Expr>,
        r: Box<Expr>,
        line: usize,
    },
    Un {
        op: UnOp,
        e: Box<Expr>,
        line: usize,
    },
    // borrow expr — emprunt EXPLICITE (optionnel : le compilateur emprunte déjà tout seul)
    Borrow(Box<Expr>, usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
}

impl Expr {
    pub fn ligne(&self) -> usize {
        match self {
            Expr::IntLit(_, l)
            | Expr::FloatLit(_, l)
            | Expr::StrLit(_, l)
            | Expr::BoolLit(_, l)
            | Expr::Ident(_, l)
            | Expr::Call { line: l, .. }
            | Expr::MethodCall { line: l, .. }
            | Expr::Field { line: l, .. }
            | Expr::Index { line: l, .. }
            | Expr::New { line: l, .. }
            | Expr::ArrayLit { line: l, .. }
            | Expr::Bin { line: l, .. }
            | Expr::Un { line: l, .. }
            | Expr::Borrow(_, l) => *l,
        }
    }
}
