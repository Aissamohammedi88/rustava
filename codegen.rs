// ------------------------------------------------------------
// RUSTAVA — Générateur de code C
// "La JVM est remplacée par un compilateur natif."
//
// Conventions du C généré :
//   - tout objet est un RvObj* (le descripteur rv_classe suit l'objet)
//   - accès champ / méthode : cast vers la classe STATIQUE connue
//   - variable typée par une INTERFACE : dispatch dynamique via
//     la table de méthodes du descripteur (comme Java, mais natif)
//   - arène déterministe : ZÉRO ramasse-miettes
// ------------------------------------------------------------

use crate::ast::*;
use std::collections::HashMap;
use std::fmt::Write as _;

pub struct Gen {
    classes: HashMap<String, ClassDecl>,
    interfaces: HashMap<String, InterfaceDecl>,
    fns: HashMap<String, FnDecl>,
    out: String,
    indent: usize,
    n_lbl: usize,
    classe_courante: Option<String>,
    self_param: String,
}

/// type C de bas niveau
#[derive(Clone, PartialEq, Debug)]
enum Tipo {
    I64,
    F64,
    Bool,
    Str,
    Obj(String), // classe OU interface
    Arr(Box<Tipo>),
    Void,
}

impl Tipo {
    fn de_type(t: &Type) -> Tipo {
        match t {
            Type::Int => Tipo::I64,
            Type::Float => Tipo::F64,
            Type::Bool => Tipo::Bool,
            Type::Str => Tipo::Str,
            Type::Arr(e) => Tipo::Arr(Box::new(Tipo::de_type(e))),
            Type::Class(n) | Type::Interface(n) => Tipo::Obj(n.clone()),
            Type::Void => Tipo::Void,
        }
    }
    fn elem(&self) -> &'static str {
        match self {
            Tipo::I64 => "RV_EL_I64",
            Tipo::F64 => "RV_EL_F64",
            Tipo::Str => "RV_EL_STR",
            Tipo::Obj(_) => "RV_EL_OBJ",
            _ => "RV_EL_I64",
        }
    }
    fn c(&self) -> String {
        match self {
            Tipo::I64 => "int64_t".into(),
            Tipo::F64 => "double".into(),
            Tipo::Bool => "bool".into(),
            Tipo::Str => "RvStr*".into(),
            // uniformité : tout objet (classe ou interface) est RvObj*,
            // les casts concrets sont posés aux points d'accès
            Tipo::Obj(_) => "RvObj*".into(),
            Tipo::Arr(_) => "RvArr*".into(),
            Tipo::Void => "void".into(),
        }
    }
}

fn c_str(s: &str) -> String {
    let mut o = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            '\0' => o.push_str("\\0"),
            c if (c as u32) < 32 => {
                let _ = write!(o, "\\{:03o}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn id(n: &str) -> String {
    format!("rv_{}", n)
}

/// contexte de typage C local (aligné sur le checker, déjà validé)
type Ctx = Vec<HashMap<String, Tipo>>;

impl Gen {
    pub fn new(prog: &Program) -> Self {
        let mut classes = HashMap::new();
        let mut interfaces = HashMap::new();
        let mut fns = HashMap::new();
        for it in &prog.items {
            match it {
                Item::Class(c) => {
                    classes.insert(c.name.clone(), c.clone());
                }
                Item::Interface(i) => {
                    interfaces.insert(i.name.clone(), i.clone());
                }
                Item::Fn(f) => {
                    fns.insert(f.name.clone(), f.clone());
                }
            }
        }
        Gen {
            classes,
            interfaces,
            fns,
            out: String::new(),
            indent: 0,
            n_lbl: 0,
            classe_courante: None,
            self_param: "self".into(),
        }
    }

    fn lbl(&mut self) -> usize {
        self.n_lbl += 1;
        self.n_lbl
    }

    fn p(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
        let _ = writeln!(self.out, "{}", s);
    }
    fn p_raw(&mut self, s: &str) {
        let _ = writeln!(self.out, "{}", s);
    }
    fn p_vide(&mut self) {
        self.out.push('\n');
    }

    fn tipo_expr(&mut self, ctx: &mut Ctx, e: &Expr) -> Tipo {
        match e {
            Expr::IntLit(..) => Tipo::I64,
            Expr::FloatLit(..) => Tipo::F64,
            Expr::BoolLit(..) => Tipo::Bool,
            Expr::StrLit(..) => Tipo::Str,
            Expr::Ident(n, _) => {
                if n == "self" {
                    return Tipo::Obj(self.classe_courante.clone().unwrap_or_default());
                }
                for b in ctx.iter().rev() {
                    if let Some(t) = b.get(n) {
                        return t.clone();
                    }
                }
                Tipo::Void
            }
            Expr::Call { callee, args, .. } => match callee.as_str() {
                "longueur" => Tipo::I64,
                "entier_vers_texte" | "caractere" => Tipo::Str,
                "texte_vers_entier" | "horloge_ms" => Tipo::I64,
                _ => {
                    if let Some(cn) = self.classe_courante.clone() {
                        if let Some(c) = self.classes.get(&cn) {
                            if let Some(m) = c.methods.iter().find(|m| m.name == *callee) {
                                return Tipo::de_type(m.ret.as_ref().unwrap_or(&Type::Void));
                            }
                        }
                    }
                    if let Some(f) = self.fns.get(callee) {
                        Tipo::de_type(f.ret.as_ref().unwrap_or(&Type::Void))
                    } else {
                        Tipo::Void
                    }
                }
            },
            Expr::MethodCall { obj, name, .. } => {
                let to = self.tipo_expr(ctx, obj);
                match to {
                    Tipo::Obj(cn) => {
                        if let Some(c) = self.classes.get(&cn) {
                            if let Some(m) = c.methods.iter().find(|m| m.name == *name) {
                                return Tipo::de_type(m.ret.as_ref().unwrap_or(&Type::Void));
                            }
                        }
                        if let Some(i) = self.interfaces.get(&cn) {
                            if let Some(sig) = i.methods.iter().find(|m| m.name == *name) {
                                return Tipo::de_type(sig.ret.as_ref().unwrap_or(&Type::Void));
                            }
                        }
                        Tipo::Void
                    }
                    Tipo::Str => {
                        if name == "longueur" {
                            Tipo::I64
                        } else {
                            Tipo::Str
                        }
                    }
                    Tipo::Arr(_) => Tipo::I64,
                    _ => Tipo::Void,
                }
            }
            Expr::Field { obj, name, .. } => {
                let to = self.tipo_expr(ctx, obj);
                if let Tipo::Obj(cn) = to {
                    if let Some(c) = self.classes.get(&cn) {
                        if let Some(f) = c.fields.iter().find(|f| f.name == *name) {
                            return Tipo::de_type(&f.ty);
                        }
                    }
                }
                Tipo::Void
            }
            Expr::Index { obj, .. } => match self.tipo_expr(ctx, obj) {
                Tipo::Arr(t) => *t,
                _ => Tipo::Void,
            },
            Expr::New { class, .. } => Tipo::Obj(class.clone()),
            Expr::ArrayLit { items, .. } => {
                if items.is_empty() {
                    Tipo::Arr(Box::new(Tipo::I64))
                } else {
                    let t0 = self.tipo_expr(ctx, &items[0]);
                    Tipo::Arr(Box::new(t0))
                }
            }
            Expr::Bin { op, l, r, .. } => {
                let tl = self.tipo_expr(ctx, l);
                let tr = self.tipo_expr(ctx, r);
                match op {
                    BinOp::Add if tl == Tipo::Str && tr == Tipo::Str => Tipo::Str,
                    BinOp::And | BinOp::Or | BinOp::Eq | BinOp::NotEq | BinOp::Lt
                    | BinOp::Le | BinOp::Gt | BinOp::Ge => Tipo::Bool,
                    _ => {
                        if tl == Tipo::F64 || tr == Tipo::F64 {
                            Tipo::F64
                        } else {
                            tl
                        }
                    }
                }
            }
            Expr::Un { op, e, .. } => {
                let t = self.tipo_expr(ctx, e);
                match op {
                    UnOp::Not => Tipo::Bool,
                    _ => t,
                }
            }
            Expr::Borrow(inner, _) => self.tipo_expr(ctx, inner),
        }
    }

    // ---------------- programme ----------------

    pub fn generer(mut self) -> String {
        self.entete();
        self.decls_classes();
        self.prototypes();
        self.descripteurs_et_fabriques();
        self.corps();
        self.main_reel();
        self.out
    }

    fn entete(&mut self) {
        self.p_raw("#include \"runtime.h\"");
        self.p_vide();
    }

    fn decls_classes(&mut self) {
        for (_n, c) in self.classes.clone() {
            self.p("// ----------------------------------------");
            let _ = writeln!(self.out, "// classe {}", c.name);
            self.p("// ----------------------------------------");
            self.p("typedef struct {");
            self.indent += 1;
            self.p("RV_TETE_OBJET;");
            for f in &c.fields {
                let t = Tipo::de_type(&f.ty);
                self.p(&format!("{} {};", t.c(), id(&f.name)));
            }
            self.indent -= 1;
            let _ = writeln!(self.out, "}} {};", c.name);
            self.p_vide();
        }
    }

    fn prototypes(&mut self) {
        for (_n, f) in self.fns.clone() {
            let ret = Tipo::de_type(f.ret.as_ref().unwrap_or(&Type::Void));
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| format!("{} {}", Tipo::de_type(&p.ty).c(), id(&p.name)))
                .collect();
            let ps = if params.is_empty() {
                "void".to_string()
            } else {
                params.join(", ")
            };
            let _ = writeln!(self.out, "static {} fn_{}({});", ret.c(), f.name, ps);
        }
        for (_n, c) in self.classes.clone() {
            for m in &c.methods {
                let ret = Tipo::de_type(m.ret.as_ref().unwrap_or(&Type::Void));
                let mut params = vec![format!("{} *{}", c.name, self.self_param)];
                for p in &m.params {
                    params.push(format!("{} {}", Tipo::de_type(&p.ty).c(), id(&p.name)));
                }
                let _ = writeln!(
                    self.out,
                    "static {} m_{}_{}_({});",
                    ret.c(),
                    c.name,
                    m.name,
                    params.join(", ")
                );
            }
        }
        self.p_vide();
    }

    /// vtable de chaque classe + fabrique new_X
    /// (c'est ici que vit le dispatch dynamique des interfaces)
    fn descripteurs_et_fabriques(&mut self) {
        for (_n, c) in self.classes.clone() {
            // 1. la table des méthodes (vtable)
            if !c.methods.is_empty() {
                let _ = writeln!(
                    self.out,
                    "static const RvMeth rv_meths_{}[] = {{",
                    c.name
                );
                for m in &c.methods {
                    let _ = writeln!(
                        self.out,
                    "    {{ \"{}\", (void*)m_{}_{}_ }},",
                        m.name, c.name, m.name
                    );
                }
                let _ = writeln!(self.out, "}};");
            }
            let nb = c.methods.len();
            if nb > 0 {
                let _ = writeln!(
                    self.out,
                    "static const RvClasse rv_desc_{0} = {{ \"{0}\", rv_meths_{0}, {1} }};",
                    c.name, nb
                );
            } else {
                let _ = writeln!(
                    self.out,
                    "static const RvClasse rv_desc_{0} = {{ \"{0}\", 0, 0 }};",
                    c.name
                );
            }
            self.p_vide();

            // 2. la fabrique
            let ctor = c.methods.iter().find(|m| m.est_constructeur(&c.name)).cloned();
            let params: Vec<String> = match &ctor {
                Some(m) => m
                    .params
                    .iter()
                    .map(|p| format!("{} {}", Tipo::de_type(&p.ty).c(), id(&p.name)))
                    .collect(),
                None => vec![],
            };
            let ps = if params.is_empty() {
                "void".to_string()
            } else {
                params.join(", ")
            };
            let _ = writeln!(self.out, "// fabrique de {}", c.name);
            let _ = writeln!(
                self.out,
                "static RvObj* new_{0}({1}) {{",
                c.name, ps
            );
            let _ = writeln!(
                self.out,
                "    {0}* o = ({0}*)rv_objet_creer(sizeof({0}));",
                c.name
            );
            let _ = writeln!(self.out, "    o->rv_classe = &rv_desc_{};", c.name);
            if let Some(m) = &ctor {
                let mut call = format!("    m_{}_{}_(o", c.name, m.name);
                for p in &m.params {
                    call.push_str(&format!(", {}", id(&p.name)));
                }
                call.push_str(");");
                let _ = writeln!(self.out, "{}", call);
            }
            let _ = writeln!(self.out, "    return (RvObj*)o;");
            let _ = writeln!(self.out, "}}");
            self.p_vide();
        }
    }

    fn corps(&mut self) {
        for (_n, f) in self.fns.clone() {
            let ret = Tipo::de_type(f.ret.as_ref().unwrap_or(&Type::Void));
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| format!("{} {}", Tipo::de_type(&p.ty).c(), id(&p.name)))
                .collect();
            let ps = if params.is_empty() {
                "void".to_string()
            } else {
                params.join(", ")
            };
            let _ = writeln!(self.out, "static {} fn_{}({}) {{", ret.c(), f.name, ps);
            self.indent += 1;
            self.classe_courante = None;
            let mut ctx: Ctx = vec![f
                .params
                .iter()
                .map(|p| (p.name.clone(), Tipo::de_type(&p.ty)))
                .collect()];
            self.bloc_c(&f.body, &mut ctx, f.ret.is_some());
            self.indent -= 1;
            self.p("}");
            self.p_vide();
        }
        for (_n, c) in self.classes.clone() {
            for m in &c.methods {
                let ret = Tipo::de_type(m.ret.as_ref().unwrap_or(&Type::Void));
                let mut params = vec![format!("{} *{}", c.name, self.self_param)];
                for p in &m.params {
                    params.push(format!("{} {}", Tipo::de_type(&p.ty).c(), id(&p.name)));
                }
                let _ = writeln!(
                    self.out,
                    "static {} m_{}_{}_({}) {{",
                    ret.c(),
                    c.name,
                    m.name,
                    params.join(", ")
                );
                self.indent += 1;
                self.classe_courante = Some(c.name.clone());
                let mut ctx: Ctx = vec![m
                    .params
                    .iter()
                    .map(|p| (p.name.clone(), Tipo::de_type(&p.ty)))
                    .collect()];
                ctx.last_mut()
                    .unwrap()
                    .insert("self".into(), Tipo::Obj(c.name.clone()));
                self.bloc_c(&m.body, &mut ctx, m.ret.is_some());
                self.indent -= 1;
                self.p("}");
                self.p_vide();
            }
        }
    }

    fn main_reel(&mut self) {
        self.p_raw("int main(void) {");
        self.indent += 1;
        self.p("rv_runtime_init();");
        self.p("RV_TRY {");
        self.indent += 1;
        self.p("fn_main();");
        self.indent -= 1;
        self.p("} else {");
        self.indent += 1;
        self.p("RvStr* e = rv_erreur_active;");
        self.p("rv_erreur_non_attrapee(e);");
        self.indent -= 1;
        self.p("}");
        self.p("return 0;");
        self.indent -= 1;
        self.p("}");
    }

    // ---------------- instructions ----------------

    fn bloc_c(&mut self, b: &Block, ctx: &mut Ctx, avec_retour: bool) {
        ctx.push(HashMap::new());
        for s in &b.stmts {
            self.instr_c(s, ctx, avec_retour);
        }
        ctx.pop();
    }

    fn instr_c(&mut self, s: &Stmt, ctx: &mut Ctx, avec_retour: bool) {
        match s {
            Stmt::Decl { name, ty, init, .. } => {
                let v = self.expr_c(init, ctx);
                let t = match ty {
                    Some(t) => Tipo::de_type(t),
                    None => self.tipo_expr(ctx, init),
                };
                ctx.last_mut().unwrap().insert(name.clone(), t.clone());
                self.p(&format!("{} {} = {};", t.c(), id(name), v));
            }
            Stmt::Assign { target, op, value, .. } => match target {
                Expr::Ident(n, _) => {
                    let val = self.expr_c(value, ctx);
                    match op {
                        None => self.p(&format!("{} = {};", id(n), val)),
                        Some(o) => {
                            let sym = match o {
                                BinOp::Add => "+",
                                BinOp::Sub => "-",
                                BinOp::Mul => "*",
                                BinOp::Div => "/",
                                _ => "%",
                            };
                            self.p(&format!("{} {}= {};", id(n), sym, val));
                        }
                    }
                }
                Expr::Field { obj, name, .. } => {
                    let oc = self.accès_objet(obj, ctx);
                    let val = self.expr_c(value, ctx);
                    match op {
                        None => self.p(&format!("{}->{} = {};", oc, id(name), val)),
                        Some(o) => {
                            let sym = match o {
                                BinOp::Add => "+",
                                BinOp::Sub => "-",
                                BinOp::Mul => "*",
                                BinOp::Div => "/",
                                _ => "%",
                            };
                            self.p(&format!("{}->{} {}= {};", oc, id(name), sym, val));
                        }
                    }
                }
                Expr::Index { obj, idx, .. } => {
                    let oc = self.expr_c(obj, ctx);
                    let ic = self.expr_c(idx, ctx);
                    let val = self.expr_c(value, ctx);
                    let t = match self.tipo_expr(ctx, obj) {
                        Tipo::Arr(te) => *te,
                        _ => Tipo::I64,
                    };
                    let op_s = match op {
                        None => "=",
                        Some(BinOp::Add) => "+=",
                        Some(BinOp::Sub) => "-=",
                        Some(BinOp::Mul) => "*=",
                        Some(BinOp::Div) => "/=",
                        _ => "%=",
                    };
                    let f = match t {
                        Tipo::I64 => "rv_arr_ecrire_i64",
                        Tipo::F64 => "rv_arr_ecrire_f64",
                        _ => "rv_arr_ecrire_str",
                    };
                    if matches!(op, Some(BinOp::Div)) {
                        self.p(&format!(
                            "if ({ic} == 0) RV_THROW(rv_str_lit(\"division par zéro\"));",
                            ic = ic
                        ));
                    }
                    self.p(&format!("{}({}, {}, \"{}\", {});", f, oc, ic, op_s, val));
                }
                _ => {}
            },
            Stmt::ExprStmt(e, _) => {
                let v = self.expr_c(e, ctx);
                if !v.is_empty() {
                    self.p(&format!("(void){};", v));
                }
            }
            Stmt::If { arms, else_blk, .. } => {
                for (i, (cond, blk)) in arms.iter().enumerate() {
                    let c = self.expr_c(cond, ctx);
                    if i == 0 {
                        self.p(&format!("if ({}) {{", c));
                    } else {
                        self.p(&format!("}} else if ({}) {{", c));
                    }
                    self.indent += 1;
                    self.bloc_c(blk, ctx, avec_retour);
                    self.indent -= 1;
                }
                if let Some(b) = else_blk {
                    self.p("} else {");
                    self.indent += 1;
                    self.bloc_c(b, ctx, avec_retour);
                    self.indent -= 1;
                }
                self.p("}");
            }
            Stmt::While { cond, body, .. } => {
                let c = self.expr_c(cond, ctx);
                self.p(&format!("while ({}) {{", c));
                self.indent += 1;
                self.bloc_c(body, ctx, avec_retour);
                self.indent -= 1;
                self.p("}");
            }
            Stmt::For { var, iter, body, .. } => {
                let nom = id(var);
                match iter {
                    Iter::Range(a, b) => {
                        let ac = self.expr_c(a, ctx);
                        let bc = self.expr_c(b, ctx);
                        ctx.last_mut().unwrap().insert(var.clone(), Tipo::I64);
                        self.p(&format!(
                            "for (int64_t {} = {}; {} < {}; {} += 1) {{",
                            nom, ac, nom, bc, nom
                        ));
                        self.indent += 1;
                        self.bloc_c(body, ctx, avec_retour);
                        self.indent -= 1;
                        self.p("}");
                    }
                    Iter::Items(e) => {
                        let ac = self.expr_c(e, ctx);
                        let elem = match self.tipo_expr(ctx, e) {
                            Tipo::Arr(te) => *te,
                            _ => Tipo::I64,
                        };
                        ctx.last_mut().unwrap().insert(var.clone(), elem.clone());
                        let n = self.lbl();
                        self.p(&format!(
                            "for (int64_t i_{n} = 0; i_{n} < rv_arr_len({a}); i_{n} += 1) {{",
                            n = n,
                            a = ac
                        ));
                        self.indent += 1;
                        let lire = match elem {
                            Tipo::I64 => format!("rv_arr_lire_i64({}, i_{})", ac, n),
                            Tipo::F64 => format!("rv_arr_lire_f64({}, i_{})", ac, n),
                            _ => format!("rv_arr_lire_str({}, i_{})", ac, n),
                        };
                        self.p(&format!("{} {} = {};", elem.c(), nom, lire));
                        self.bloc_c(body, ctx, avec_retour);
                        self.indent -= 1;
                        self.p("}");
                    }
                }
            }
            Stmt::Match { subject, arms, .. } => {
                let sc = self.expr_c(subject, ctx);
                let t = self.tipo_expr(ctx, subject);
                let n = self.lbl();
                let var_s = format!("sujet_{}", n);
                self.p(&format!("{} {} = {};", t.c(), var_s, sc));
                for (i, a) in arms.iter().enumerate() {
                    let cond = match &a.pat {
                        MatchPat::IntLit(v) => format!("{} == {}", var_s, v),
                        MatchPat::FloatLit(v) => format!("{} == {}", var_s, v),
                        MatchPat::StrLit(v) => format!("rv_str_egal({}, rv_str_lit({}))", var_s, c_str(v)),
                        MatchPat::BoolLit(v) => format!("{} == {}", var_s, v),
                        MatchPat::Wildcard => "1".to_string(),
                    };
                    if i == 0 {
                        self.p(&format!("if ({}) {{", cond));
                    } else {
                        self.p(&format!("}} else if ({}) {{", cond));
                    }
                    self.indent += 1;
                    self.bloc_c(&a.body, ctx, avec_retour);
                    self.indent -= 1;
                }
                self.p("}");
            }
            Stmt::Try { body, catch_var, catch_blk, .. } => {
                self.p("RV_TRY {");
                self.indent += 1;
                self.bloc_c(body, ctx, avec_retour);
                self.indent -= 1;
                self.p("} else {");
                self.indent += 1;
                if let Some(v) = catch_var {
                    self.p(&format!("RvStr* {} = rv_erreur_active;", id(v)));
                    ctx.last_mut().unwrap().insert(v.clone(), Tipo::Str);
                } else {
                    self.p("(void)rv_erreur_active;");
                }
                self.p("rv_pile_prof--;");
                self.bloc_c(catch_blk, ctx, avec_retour);
                self.indent -= 1;
                self.p("}");
            }
            Stmt::Throw(e, _) => {
                let v = self.expr_c(e, ctx);
                self.p(&format!("RV_THROW({});", v));
            }
            Stmt::Return(e, _) => match e {
                Some(x) => {
                    let v = self.expr_c(x, ctx);
                    self.p(&format!("return {};", v));
                }
                None => self.p("return;"),
            },
            Stmt::Spawn(e, _) => {
                if let Expr::Call { callee, args, .. } = e {
                    if args.is_empty() {
                        self.p(&format!("rv_spawn_0((void(*)(void))fn_{});", callee));
                    } else {
                        // spawn accepte 0 ou 1 argument simple (int ou string)
                        let t = self.tipo_expr(ctx, &args[0]);
                        let a = self.expr_c(&args[0], ctx);
                        match t {
                            Tipo::Str => {
                                self.p(&format!(
                                    "rv_spawn_s((void(*)(RvStr*))fn_{}, {});",
                                    callee, a
                                ));
                            }
                            _ => {
                                self.p(&format!(
                                    "rv_spawn_i((void(*)(int64_t))fn_{}, {});",
                                    callee, a
                                ));
                            }
                        }
                    }
                }
                // spawn d'une méthode : refusé par le checker (message clair)
            }
            Stmt::Block(b, _) => {
                self.bloc_c(b, ctx, avec_retour);
            }
        }
    }

    /// expression objet → texte C ; si le type statique est une CLASSE,
    /// renvoie le cast concret pour l'accès aux champs/méthodes
    fn accès_objet(&mut self, obj: &Expr, ctx: &mut Ctx) -> String {
        let base = self.expr_c(obj, ctx);
        let t = self.tipo_expr(ctx, obj);
        if let Tipo::Obj(cn) = t {
            if self.classes.contains_key(&cn) {
                return format!("(({}*){})", cn, base);
            }
        }
        base
    }

    // ---------------- expressions ----------------

    fn expr_c(&mut self, e: &Expr, ctx: &mut Ctx) -> String {
        match e {
            Expr::IntLit(v, _) => format!("{}", v),
            Expr::FloatLit(v, _) => {
                if *v == v.trunc() && v.is_finite() {
                    format!("{:.1}", v)
                } else {
                    format!("{}", v)
                }
            }
            Expr::BoolLit(v, _) => format!("{}", v),
            Expr::StrLit(s, _) => format!("rv_str_lit({})", c_str(s)),
            Expr::Ident(n, _) => {
                if n == "self" {
                    self.self_param.clone()
                } else {
                    id(n)
                }
            }
            Expr::Borrow(inner, _) => self.expr_c(inner, ctx),
            Expr::Call { callee, args, .. } => match callee.as_str() {
                "print" | "afficher" => {
                    let a = self.expr_c(&args[0], ctx);
                    let t = self.tipo_expr(ctx, &args[0]);
                    let f = match t {
                        Tipo::I64 => "rv_afficher_int",
                        Tipo::F64 => "rv_afficher_float",
                        Tipo::Bool => "rv_afficher_bool",
                        Tipo::Str => "rv_afficher_str",
                        Tipo::Arr(_) => "rv_afficher_obj",
                        _ => "rv_afficher_obj",
                    };
                    let arg = match t {
                        Tipo::Arr(_) => format!("(RvObj*){}", a),
                        _ => a,
                    };
                    format!("{}({})", f, arg)
                }
                "longueur" => {
                    let a = self.expr_c(&args[0], ctx);
                    format!("rv_longueur((RvObj*){})", a)
                }
                "caractere" => {
                    let a = self.expr_c(&args[0], ctx);
                    let i = self.expr_c(&args[1], ctx);
                    format!("rv_caractere({}, {})", a, i)
                }
                "entier_vers_texte" => {
                    let a = self.expr_c(&args[0], ctx);
                    format!("rv_str_de_int({})", a)
                }
                "texte_vers_entier" => {
                    let a = self.expr_c(&args[0], ctx);
                    format!("rv_str_vers_int({})", a)
                }
                "horloge_ms" => "rv_horloge_ms()".into(),
                "dormir_ms" => {
                    let a = self.expr_c(&args[0], ctx);
                    format!("rv_dormir_ms({})", a)
                }
                _ => {
                    if let Some(cn) = self.classe_courante.clone() {
                        if let Some(c) = self.classes.get(&cn) {
                            if c.methods.iter().any(|m| m.name == *callee) {
                                let mut call =
                                    format!("m_{}_{}_(({}*){})", cn, callee, cn, self.self_param);
                                for a in args {
                                    call.push_str(&format!(", {}", self.expr_c(a, ctx)));
                                }
                                call.push(')');
                                return call;
                            }
                        }
                    }
                    let mut call = format!("fn_{}(", callee);
                    let pieces: Vec<String> =
                        args.iter().map(|a| self.expr_c(a, ctx)).collect();
                    call.push_str(&pieces.join(", "));
                    call.push(')');
                    call
                }
            },
            Expr::MethodCall { obj, name, args, .. } => {
                let oc = self.expr_c(obj, ctx);
                let t_obj = self.tipo_expr(ctx, obj);
                match t_obj {
                    Tipo::Obj(cn) => {
                        // CLASSE concrète → appel statique, avec cast
                        if self.classes.contains_key(&cn) {
                            let mut call = format!(
                                "m_{}_{}_((({c}*){o})",
                                cn, name,
                                c = cn,
                                o = oc
                            );
                            for a in args {
                                call.push_str(&format!(", {}", self.expr_c(a, ctx)));
                            }
                            call.push(')');
                            return call;
                        }
                        // INTERFACE → dispatch dynamique via le descripteur
                        // (toutes les implémentations ont la même signature : le checker l'a vérifié)
                        if let Some(i) = self.interfaces.get(&cn) {
                            if let Some(sig) = i.methods.iter().find(|m| m.name == *name) {
                                let ret =
                                    Tipo::de_type(sig.ret.as_ref().unwrap_or(&Type::Void));
                                let mut sigc = String::from("RvObj*");
                                let mut pieces = vec![oc.clone()];
                                for p in &sig.params {
                                    sigc.push_str(&format!(", {}", Tipo::de_type(&p.ty).c()));
                                }
                                for a in args {
                                    pieces.push(self.expr_c(a, ctx));
                                }
                                let cast = match ret {
                                    Tipo::Void => format!(
                                        "((void(*)({}))rv_methode(({})->rv_classe, \"{}\"))",
                                        sigc, oc, name
                                    ),
                                    _ => format!(
                                        "(({}(*)({}))rv_methode(({})->rv_classe, \"{}\"))",
                                        ret.c(),
                                        sigc,
                                        oc,
                                        name
                                    ),
                                };
                                let mut call = cast;
                                call.push_str(&format!("({})", pieces.join(", ")));
                                return call;
                            }
                        }
                        oc
                    }
                    Tipo::Str => match name.as_str() {
                        "longueur" | "taille" => format!("rv_longueur((RvObj*){})", oc),
                        "majuscule" => format!("rv_str_majuscule({})", oc),
                        _ => format!("rv_longueur((RvObj*){})", oc),
                    },
                    Tipo::Arr(_) => format!("rv_arr_len({})", oc),
                    _ => oc,
                }
            }
            Expr::Field { obj, name, .. } => {
                let acc = self.accès_objet(obj, ctx);
                format!("{}->{}", acc, id(name))
            }
            Expr::Index { obj, idx, .. } => {
                let oc = self.expr_c(obj, ctx);
                let ic = self.expr_c(idx, ctx);
                let t = match self.tipo_expr(ctx, obj) {
                    Tipo::Arr(te) => *te,
                    _ => Tipo::I64,
                };
                let f = match t {
                    Tipo::I64 => "rv_arr_lire_i64",
                    Tipo::F64 => "rv_arr_lire_f64",
                    _ => "rv_arr_lire_str",
                };
                format!("{}({}, {})", f, oc, ic)
            }
            Expr::New { class, args, .. } => {
                let mut call = format!("new_{}(", class);
                let pieces: Vec<String> =
                    args.iter().map(|a| self.expr_c(a, ctx)).collect();
                call.push_str(&pieces.join(", "));
                call.push(')');
                call
            }
            Expr::ArrayLit { items, .. } => {
                let mut pieces = vec![];
                for it in items {
                    pieces.push(self.expr_c(it, ctx));
                }
                let t = if items.is_empty() {
                    Tipo::I64
                } else {
                    match self.tipo_expr(ctx, &items[0]) {
                        Tipo::Arr(te) => *te,
                        t => t,
                    }
                };
                let n = pieces.len();
                let vals = if n == 0 {
                    String::new()
                } else {
                    pieces
                        .iter()
                        .map(|p| format!("rv_v_i64({})", p))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let el = t.elem();
                format!(
                    "rv_arr_lit({el}, {n}, (RVVal[]){{ {vals} }})",
                    el = el,
                    n = n,
                    vals = vals
                )
            }
            Expr::Bin { op, l, r, .. } => {
                let lc = self.expr_c(l, ctx);
                let rc = self.expr_c(r, ctx);
                let tl = self.tipo_expr(ctx, l);
                match op {
                    BinOp::Add if tl == Tipo::Str => {
                        format!("rv_str_concat({}, {})", lc, rc)
                    }
                    BinOp::And => format!("({} && {})", lc, rc),
                    BinOp::Or => format!("({} || {})", lc, rc),
                    BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        if tl == Tipo::Str {
                            let neg = if *op == BinOp::NotEq { "!" } else { "" };
                            format!("{}rv_str_egal({}, {})", neg, lc, rc)
                        } else {
                            let sym = match op {
                                BinOp::Eq => "==",
                                BinOp::NotEq => "!=",
                                BinOp::Lt => "<",
                                BinOp::Le => "<=",
                                BinOp::Gt => ">",
                                _ => ">=",
                            };
                            format!("({} {} {})", lc, sym, rc)
                        }
                    }
                    _ => {
                        // division / modulo : panique Rustava attrapable (comme Rust)
                        if *op == BinOp::Div || *op == BinOp::Mod {
                            let t = self.tipo_expr(ctx, l);
                            let f = match (*op, t) {
                                (BinOp::Div, Tipo::F64) => "rv_div_f64",
                                (BinOp::Div, _) => "rv_div_i64",
                                (_, Tipo::F64) => "rv_mod_f64",
                                _ => "rv_mod_i64",
                            };
                            format!("{}({}, {})", f, lc, rc)
                        } else {
                            let sym = match op {
                                BinOp::Add => "+",
                                BinOp::Sub => "-",
                                BinOp::Mul => "*",
                                _ => "%",
                            };
                            format!("({} {} {})", lc, sym, rc)
                        }
                    }
                }
            }
            Expr::Un { op, e, .. } => {
                let c = self.expr_c(e, ctx);
                match op {
                    UnOp::Neg => format!("(-{})", c),
                    UnOp::Not => format!("(!{})", c),
                }
            }
        }
    }
}
