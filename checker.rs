// ------------------------------------------------------------
// RUSTAVA — Type Checker + Borrow Checker simplifié
// "Le compilateur gère la mémoire, l'utilisateur ne se bat pas."
//   1. let immuable / var mutable  (à la compilation, comme Rust)
//   2. interfaces : toutes les méthodes présentes (contrat Java)
//   3. propriétaires inférés : passage par référence automatique
// ------------------------------------------------------------

use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ErreurType {
    pub msg: String,
    pub line: usize,
}

type R<T> = Result<T, ErreurType>;

#[derive(Clone, PartialEq, Debug)]
pub enum Valeur {
    Int,
    Float,
    Bool,
    Str,
    Arr(Box<TType>),
    Obj(String), // nom de classe (les interfaces sont vérifiées à part)
    Void,
}

pub type TType = Valeur;

#[derive(Clone)]
struct VarInfo {
    ty: Valeur,
    mutable: bool,
}

// contexte d'une fonction en cours de vérification
#[derive(Clone)]
struct Ctx {
    locals: Vec<HashMap<String, VarInfo>>,
    // pile de blocs ; le bas = paramètres de la fonction
    classe: Option<String>, // si on vérifie une méthode, la classe englobante
}

pub struct Checker {
    classes: HashMap<String, ClassDecl>,
    interfaces: HashMap<String, InterfaceDecl>,
    fns: HashMap<String, FnDecl>,
    erreurs: Vec<ErreurType>,
    /// fonctions et méthodes intrinsèques (print, longueur...)
    intrinseques: HashMap<String, (usize, Option<Valeur>)>,
}

fn attendu_trouve(att: &Valeur, tro: &Valeur) -> String {
    format!("attendu « {} », trouvé « {} »", aff(att), aff(tro))
}

pub fn aff(t: &Valeur) -> String {
    match t {
        Valeur::Int => "int".into(),
        Valeur::Float => "float".into(),
        Valeur::Bool => "bool".into(),
        Valeur::Str => "string".into(),
        Valeur::Arr(t) => format!("{}[]", aff(t)),
        Valeur::Obj(c) => c.clone(),
        Valeur::Void => "void".into(),
    }
}

fn de_type(t: &Type) -> Valeur {
    match t {
        Type::Int => Valeur::Int,
        Type::Float => Valeur::Float,
        Type::Bool => Valeur::Bool,
        Type::Str => Valeur::Str,
        Type::Arr(e) => Valeur::Arr(Box::new(de_type(e))),
        Type::Class(n) => Valeur::Obj(n.clone()),
        Type::Interface(n) => Valeur::Obj(n.clone()),
        Type::Void => Valeur::Void,
    }
}

impl Checker {
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
        let mut intrinseques = HashMap::new();
        intrinseques.insert("print".into(), (1, Some(Valeur::Void)));
        intrinseques.insert("afficher".into(), (1, Some(Valeur::Void)));
        intrinseques.insert("longueur".into(), (1, Some(Valeur::Int)));
        intrinseques.insert("caractere".into(), (2, Some(Valeur::Str)));
        intrinseques.insert(
            "entier_vers_texte".into(),
            (1, Some(Valeur::Str)),
        );
        intrinseques.insert(
            "texte_vers_entier".into(),
            (1, Some(Valeur::Int)),
        );
        intrinseques.insert("horloge_ms".into(), (0, Some(Valeur::Int)));
        intrinseques.insert("dormir_ms".into(), (1, Some(Valeur::Void)));
        Checker {
            classes,
            interfaces,
            fns,
            erreurs: Vec::new(),
            intrinseques,
        }
    }

    fn classes_contiennent_methode(&self, nom: &str) -> bool {
        self.classes.values().any(|c| c.methods.iter().any(|m| m.name == nom))
    }

    pub fn verifier_tout(&mut self) -> R<()> {
        // 0. unicité des noms (classe vs interface vs fn)
        let mut vus: HashMap<String, usize> = HashMap::new();
        for (n, c) in &self.classes {
            vus.insert(n.clone(), c.line);
        }
        for (n, i) in &self.interfaces {
            if vus.contains_key(n) {
                return Err(ErreurType {
                    msg: format!("« {} » est déclaré à la fois classe et interface", n),
                    line: *vus.get(n).unwrap(),
                });
            }
            vus.insert(n.clone(), i.line);
        }
        for (n, f) in &self.fns {
            if vus.contains_key(n) {
                return Err(ErreurType {
                    msg: format!(
                        "« {} » est déjà un nom de classe/interface ; la fonction doit porter un autre nom",
                        n
                    ),
                    line: f.line,
                });
            }
            vus.insert(n.clone(), f.line);
        }
        // 1. vérifier chaque classe
        let classes: Vec<ClassDecl> = self.classes.values().cloned().collect();
        for c in &classes {
            self.verifier_classe(c)?;
        }
        // 3. interfaces : méthodes uniques
        let interfaces: Vec<InterfaceDecl> = self.interfaces.values().cloned().collect();
        for i in &interfaces {
            let mut vus = HashMap::new();
            for m in &i.methods {
                if vus.contains_key(&m.name) {
                    return Err(ErreurType {
                        msg: format!(
                            "l'interface « {} » déclare deux fois la méthode « {} »",
                            i.name, m.name
                        ),
                        line: m.line,
                    });
                }
                vus.insert(m.name.clone(), ());
            }
        }
        // 4. fonctions top-niveau
        let fns: Vec<FnDecl> = self.fns.values().cloned().collect();
        for f in &fns {
            self.verifier_fn(f)?;
        }
        // 5. point d'entrée
        if !self.fns.contains_key("main") {
            return Err(ErreurType {
                msg: "pas de point d'entrée : Rustava attend une fonction « fn main() »".into(),
                line: 0,
            });
        }
        let m = self.fns.get("main").unwrap();
        if !m.params.is_empty() {
            return Err(ErreurType {
                msg: "« fn main() » ne prend aucun paramètre".into(),
                line: m.line,
            });
        }
        if m.ret.is_some() {
            return Err(ErreurType {
                msg: "« fn main() » ne retourne rien (pas de « -> type »)".into(),
                line: m.line,
            });
        }
        if !self.erreurs.is_empty() {
            let e = &self.erreurs[0];
            return Err(e.clone());
        }
        Ok(())
    }

    fn err_l(&mut self, msg: String, line: usize) {
        self.erreurs.push(ErreurType { msg, line });
    }

    fn doit_etre(&mut self, obtenu: &Valeur, attendu: &Valeur, quoi: &str, line: usize) {
        // compatibilité classe → interface (le polymorphisme du mariage) :
        // un Chien est un Animal si Chien implements Animal
        if obtenu != attendu {
            if let (Valeur::Obj(cl), Valeur::Obj(iface)) = (obtenu, attendu) {
                if let Some(cd) = self.classes.get(cl) {
                    if cd.interfaces.iter().any(|i| i == iface) {
                        return; // élargissement valide
                    }
                }
            }
            let msg = format!("{} : {}", quoi, attendu_trouve(attendu, obtenu));
            self.err_l(msg, line);
        }
    }

    fn verifier_classe(&mut self, c: &ClassDecl) -> R<()> {
        // interfaces inconnues ?
        for i in &c.interfaces {
            if !self.interfaces.contains_key(i) {
                return Err(ErreurType {
                    msg: format!(
                        "la classe « {} » implémente « {} » qui n'existe pas (vérifie l'orthographe ou déclare l'interface)",
                        c.name, i
                    ),
                    line: c.line,
                });
            }
        }
        // champs : noms uniques, types valides
        let mut vus = HashMap::new();
        for f in &c.fields {
            if vus.contains_key(&f.name) {
                return Err(ErreurType {
                    msg: format!("deux champs portent le nom « {} » dans la classe « {} »", f.name, c.name),
                    line: f.line,
                });
            }
            vus.insert(f.name.clone(), ());
            self.type_valide(&f.ty, f.line)?;
        }
        // méthodes : noms uniques (hors surcharge, Rustava est simple), signatures valides
        let mut vus_m = HashMap::new();
        for m in &c.methods {
            if vus_m.contains_key(&m.name) {
                return Err(ErreurType {
                    msg: format!(
                        "deux méthodes portent le nom « {} » dans la classe « {} » (Rustava n'a pas de surcharge : donne-leur des noms différents)",
                        m.name, c.name
                    ),
                    line: m.line,
                });
            }
            vus_m.insert(m.name.clone(), ());
            for p in &m.params {
                self.type_valide(&p.ty, p.line)?;
            }
            if let Some(rt) = &m.ret {
                self.type_valide(rt, m.line)?;
            }
        }
        // le contrat des interfaces : TOUTES les méthodes, avec la bonne signature
        for iname in &c.interfaces.clone() {
            let iface = self.interfaces.get(iname).unwrap().clone();
            for sig in &iface.methods {
                let m = c.methods.iter().find(|m| m.name == sig.name);
                let m = match m {
                    Some(m) => m,
                    None => {
                        return Err(ErreurType {
                            msg: format!(
                                "la classe « {} » dit « implements {} » mais la méthode « {} » manque (contrat incomplet)",
                                c.name, iname, sig.name
                            ),
                            line: c.line,
                        })
                    }
                };
                if m.params.len() != sig.params.len() {
                    return Err(ErreurType {
                        msg: format!(
                            "« {}::{} » prend {} paramètre(s) ; l'interface « {} » en exige {}",
                            c.name, m.name, m.params.len(), iname, sig.params.len()
                        ),
                        line: m.line,
                    });
                }
                for (a, b) in m.params.iter().zip(sig.params.iter()) {
                    if de_type(&a.ty) != de_type(&b.ty) {
                        return Err(ErreurType {
                            msg: format!(
                                "paramètre « {} » de « {}::{} » : attendu « {} » (interface « {} »), trouvé « {} »",
                                a.name, c.name, m.name, aff(&de_type(&b.ty)), iname, aff(&de_type(&a.ty))
                            ),
                            line: m.line,
                        });
                    }
                }
                if de_type(m.ret.as_ref().unwrap_or(&Type::Void))
                    != de_type(sig.ret.as_ref().unwrap_or(&Type::Void))
                {
                    return Err(ErreurType {
                        msg: format!(
                            "retour de « {}::{} » : attendu « {} » (interface « {} »), trouvé « {} »",
                            c.name, m.name,
                            aff(&de_type(sig.ret.as_ref().unwrap_or(&Type::Void))),
                            iname,
                            aff(&de_type(m.ret.as_ref().unwrap_or(&Type::Void)))
                        ),
                        line: m.line,
                    });
                }
            }
        }
        // corps des méthodes
        for m in &c.methods {
            self.verifier_methode(c, m)?;
        }
        Ok(())
    }

    fn verifier_methode(&mut self, c: &ClassDecl, m: &MethodDecl) -> R<()> {
        let mut env = HashMap::new();
        for p in &m.params {
            env.insert(p.name.clone(), VarInfo { ty: de_type(&p.ty), mutable: true });
        }
        let ctx = Ctx {
            locals: vec![env],
            classe: Some(c.name.clone()),
        };
        let retour_attendu = de_type(m.ret.as_ref().unwrap_or(&Type::Void));
        self.verifier_corps(ctx, &m.body, &retour_attendu)?;
        Ok(())
    }

    fn verifier_fn(&mut self, f: &FnDecl) -> R<()> {
        let mut env = HashMap::new();
        for p in &f.params {
            env.insert(p.name.clone(), VarInfo { ty: de_type(&p.ty), mutable: true });
        }
        let ctx = Ctx {
            locals: vec![env],
            classe: None,
        };
        let retour_attendu = de_type(f.ret.as_ref().unwrap_or(&Type::Void));
        self.verifier_corps(ctx, &f.body, &retour_attendu)?;
        Ok(())
    }

    fn type_valide(&mut self, t: &Type, line: usize) -> R<()> {
        match t {
            Type::Int | Type::Float | Type::Bool | Type::Str | Type::Void => Ok(()),
            Type::Arr(e) => self.type_valide(e, line),
            Type::Class(n) | Type::Interface(n) => {
                if self.classes.contains_key(n) || self.interfaces.contains_key(n) {
                    Ok(())
                } else {
                    Err(ErreurType {
                        msg: format!("le type « {} » n'existe pas (ni classe, ni interface)", n),
                        line,
                    })
                }
            }
        }
    }

    fn chercher(&self, ctx: &Ctx, nom: &str) -> Option<VarInfo> {
        for b in ctx.locals.iter().rev() {
            if let Some(v) = b.get(nom) {
                return Some(v.clone());
            }
        }
        None
    }

    fn verifier_corps(&mut self, mut ctx: Ctx, bloc: &Block, retour: &Valeur) -> R<()> {
        ctx.locals.push(HashMap::new());
        for s in &bloc.stmts {
            self.verifier_stmt(&mut ctx, s, retour)?;
        }
        ctx.locals.pop();
        Ok(())
    }

    fn verifier_stmt(&mut self, ctx: &mut Ctx, s: &Stmt, retour: &Valeur) -> R<()> {
        match s {
            Stmt::Decl { mutable, name, ty, init, line } => {
                let vt = self.expr(ctx, init)?;
                if let Some(ty_annonce) = ty {
                    self.type_valide(ty_annonce, *line)?;
                    if vt != de_type(ty_annonce) {
                        let msg = format!(
                            "la variable « {} » annonce « {} » mais l'expression donne « {} »",
                            name,
                            aff(&de_type(ty_annonce)),
                            aff(&vt)
                        );
                        return Err(ErreurType { msg, line: *line });
                    }
                }
                if vt == Valeur::Void {
                    return Err(ErreurType {
                        msg: format!(
                            "on ne peut pas mettre le résultat « void » de l'appel dans la variable « {} »",
                            name
                        ),
                        line: *line,
                    });
                }
                ctx.locals
                    .last_mut()
                    .unwrap()
                    .insert(name.clone(), VarInfo { ty: vt, mutable: *mutable });
                Ok(())
            }
            Stmt::Assign { target, op, value, line } => {
                let vt_cible = self.expr(ctx, target)?;
                let vt_val = self.expr(ctx, value)?;
                // mutabilité : le cœur du mariage
                if let Expr::Ident(n, _) = target {
                    let info = self.chercher(ctx, n);
                    if let Some(info) = info {
                        if !info.mutable {
                            return Err(ErreurType {
                                msg: format!(
                                    "« {} » est immuable (déclaré avec let) : impossible de la modifier — utilise var si tu veux la changer",
                                    n
                                ),
                                line: *line,
                            });
                        }
                    } else {
                        return Err(ErreurType {
                            msg: format!("je ne connais pas « {} » (déclare-la avec let ou var)", n),
                            line: *line,
                        });
                    }
                }
                if let Some(op) = op {
                    let attendu = match (vt_cible.clone(), *op) {
                        (Valeur::Str, BinOp::Add) => Valeur::Str,
                        _ => vt_cible.clone(),
                    };
                    self.doit_etre(&vt_val, &attendu, "valeur de l'affectation composée", *line);
                    // int += float interdit (comme Rust, pas de mélange implicite)
                    if vt_cible == Valeur::Int && vt_val == Valeur::Float {
                        return Err(ErreurType {
                            msg: "mélange interdit : int += float — convertis d'abord (x = x + y * 1.0 n'existe pas non plus : choisis un seul type)".into(),
                            line: *line,
                        });
                    }
                } else {
                    self.doit_etre(&vt_val, &vt_cible, "affectation", *line);
                }
                Ok(())
            }
            Stmt::ExprStmt(e, line) => {
                self.expr(ctx, e)?;
                let _ = line;
                Ok(())
            }
            Stmt::If { arms, else_blk, line } => {
                for (c, b) in arms {
                    let vc = self.expr(ctx, c)?;
                    self.doit_etre(&vc, &Valeur::Bool, "la condition du if doit être un bool", *line);
                    self.verifier_corps(ctx.clone(), b, retour)?;
                }
                if let Some(b) = else_blk {
                    self.verifier_corps(ctx.clone(), b, retour)?;
                }
                Ok(())
            }
            Stmt::While { cond, body, line } => {
                let vc = self.expr(ctx, cond)?;
                self.doit_etre(&vc, &Valeur::Bool, "la condition du while doit être un bool", *line);
                self.verifier_corps(ctx.clone(), body, retour)?;
                Ok(())
            }
            Stmt::For { var, iter, body, line } => {
                let it = match iter {
                    Iter::Range(a, b) => {
                        let va = self.expr(ctx, a)?;
                        let vb = self.expr(ctx, b)?;
                        self.doit_etre(&va, &Valeur::Int, "le début de a..b doit être un int", *line);
                        self.doit_etre(&vb, &Valeur::Int, "la fin de a..b doit être un int", *line);
                        Valeur::Int
                    }
                    Iter::Items(e) => {
                        let ve = self.expr(ctx, e)?;
                        match ve {
                            Valeur::Arr(t) => *t,
                            Valeur::Str => {
                                return Err(ErreurType {
                                    msg: "for in sur une string n'existe pas encore ; utilise caractere(s, i) et longueur(s)".into(),
                                    line: *line,
                                })
                            }
                            autre => {
                                return Err(ErreurType {
                                    msg: format!(
                                        "for in attend un tableau, trouvé « {} »",
                                        aff(&autre)
                                    ),
                                    line: *line,
                                })
                            }
                        }
                    }
                };
                ctx.locals.push(HashMap::new());
                ctx.locals.last_mut().unwrap().insert(
                    var.clone(),
                    VarInfo { ty: it, mutable: false }, // la variable de boucle est immuable, comme Rust
                );
                for s in &body.stmts {
                    self.verifier_stmt(ctx, s, retour)?;
                }
                ctx.locals.pop();
                Ok(())
            }
            Stmt::Match { subject, arms, line } => {
                let vs = self.expr(ctx, subject)?;
                for a in arms {
                    // le motif doit avoir le même type que le sujet
                    let mt = match &a.pat {
                        MatchPat::IntLit(_) => Valeur::Int,
                        MatchPat::FloatLit(_) => Valeur::Float,
                        MatchPat::StrLit(_) => Valeur::Str,
                        MatchPat::BoolLit(_) => Valeur::Bool,
                        MatchPat::Wildcard => vs.clone(),
                    };
                    if mt != vs && !matches!(a.pat, MatchPat::Wildcard) {
                        return Err(ErreurType {
                            msg: format!(
                                "le motif du match ne correspond pas au sujet : {}",
                                attendu_trouve(&vs, &mt)
                            ),
                            line: a.line,
                        });
                    }
                    self.verifier_corps(ctx.clone(), &a.body, retour)?;
                }
                if arms.is_empty() {
                    return Err(ErreurType {
                        msg: "un match doit avoir au moins une branche (n'oublie pas « _ => ... »)".into(),
                        line: *line,
                    });
                }
                Ok(())
            }
            Stmt::Try { body, catch_var, catch_blk, line } => {
                self.verifier_corps(ctx.clone(), body, retour)?;
                ctx.locals.push(HashMap::new());
                if let Some(v) = catch_var {
                    // la variable d'erreur est une string (message), immuable comme Rust
                    ctx.locals.last_mut().unwrap().insert(
                        v.clone(),
                        VarInfo { ty: Valeur::Str, mutable: false },
                    );
                }
                for s in &catch_blk.stmts {
                    self.verifier_stmt(ctx, s, retour)?;
                }
                ctx.locals.pop();
                let _ = line;
                Ok(())
            }
            Stmt::Throw(e, line) => {
                let ve = self.expr(ctx, e)?;
                self.doit_etre(&ve, &Valeur::Str, "throw attend une string (le message d'erreur)", *line);
                Ok(())
            }
            Stmt::Return(e, line) => {
                if *retour == Valeur::Void {
                    return Err(ErreurType {
                        msg: "cette fonction ne retourne rien (void) mais contient un return avec valeur".into(),
                        line: *line,
                    });
                }
                if let Some(e) = e {
                    let ve = self.expr(ctx, e)?;
                    self.doit_etre(&ve, retour, "valeur de retour", *line);
                } else {
                    return Err(ErreurType {
                        msg: format!("le return doit rendre un « {} »", aff(retour)),
                        line: *line,
                    });
                }
                Ok(())
            }
            Stmt::Spawn(e, line) => {
                // spawn : léger et asynchrone — mais seulement une FONCTION
                // top-niveau, sans objet (les threads ne courent pas sur les
                // champs d'une classe, c'est la couture choisie du mariage)
                if let Expr::Call { callee, args, .. } = e {
                    if self.classes_contiennent_methode(callee) {
                        return Err(ErreurType {
                            msg: format!(
                                "spawn n'accepte pas la méthode « {} » (elle vit dans une classe) : déplace-la en fonction top-niveau fn {}(...)",
                                callee, callee
                            ),
                            line: *line,
                        });
                    }
                    // vérifier la signature comme un appel normal
                    self.appel(ctx, callee, args, *line)?;
                    if args.len() > 1 {
                        return Err(ErreurType {
                            msg: "spawn accepte au maximum 1 argument (int ou string)".into(),
                            line: *line,
                        });
                    }
                    return Ok(());
                }
                Err(ErreurType {
                    msg: "spawn attend un appel de fonction top-niveau, par exemple : spawn afficher(\"coucou\");".into(),
                    line: *line,
                })
            }
            Stmt::Block(b, _) => {
                self.verifier_corps(ctx.clone(), b, retour)?;
                Ok(())
            }
        }
    }

    fn expr(&mut self, ctx: &mut Ctx, e: &Expr) -> R<Valeur> {
        match e {
            Expr::IntLit(_, _) => Ok(Valeur::Int),
            Expr::FloatLit(_, _) => Ok(Valeur::Float),
            Expr::BoolLit(_, _) => Ok(Valeur::Bool),
            Expr::StrLit(_, _) => Ok(Valeur::Str),
            Expr::Ident(n, line) => {
                if let Some(info) = self.chercher(ctx, n) {
                    Ok(info.ty)
                } else if n == "self" && ctx.classe.is_some() {
                    Ok(Valeur::Obj(ctx.classe.clone().unwrap()))
                } else {
                    Err(ErreurType {
                        msg: format!("je ne connais pas « {} » dans ce contexte", n),
                        line: *line,
                    })
                }
            }
            Expr::Call { callee, args, line } => self.appel(ctx, callee, args, *line),
            Expr::MethodCall { obj, name, args, line } => {
                let vo = self.expr(ctx, obj)?;
                match vo {
                    Valeur::Obj(ref classe) => {
                        // CLASSE concrète : appel statique
                        if let Some(c) = self.classes.get(classe).cloned() {
                            let m = c.methods.iter().find(|m| m.name == *name).cloned();
                            let m = match m {
                                Some(m) => m,
                                None => {
                                    if *name == "longueur" || *name == "taille" {
                                        self.verifier_args_intrinseque_obj(&vo, args, *line, &name)?;
                                        return Ok(Valeur::Int);
                                    }
                                    return Err(ErreurType {
                                        msg: format!(
                                            "la classe « {} » n'a pas de méthode « {} »",
                                            classe, name
                                        ),
                                        line: *line,
                                    });
                                }
                            };
                            self.verifier_signature(&m.params, &m.ret, args, ctx, *line)?;
                            return Ok(de_type(m.ret.as_ref().unwrap_or(&Type::Void)));
                        }
                        // INTERFACE : appel dynamique — l'objet concret décidera à l'exécution
                        if let Some(i) = self.interfaces.get(classe).cloned() {
                            let sig = i.methods.iter().find(|m| m.name == *name).cloned();
                            let sig = match sig {
                                Some(s) => s,
                                None => {
                                    return Err(ErreurType {
                                        msg: format!(
                                            "l'interface « {} » n'offre pas de méthode « {} »",
                                            classe, name
                                        ),
                                        line: *line,
                                    })
                                }
                            };
                            self.verifier_signature(&sig.params, &sig.ret, args, ctx, *line)?;
                            return Ok(de_type(sig.ret.as_ref().unwrap_or(&Type::Void)));
                        }
                        Err(ErreurType {
                            msg: format!("le type « {} » n'existe pas", classe),
                            line: *line,
                        })
                    }
                    Valeur::Str => {
                        match name.as_str() {
                            "longueur" | "taille" => {
                                if !args.is_empty() {
                                    return Err(ErreurType {
                                        msg: "s.longueur() ne prend aucun argument".into(),
                                        line: *line,
                                    });
                                }
                                Ok(Valeur::Int)
                            }
                            "majuscule" => {
                                if !args.is_empty() {
                                    return Err(ErreurType {
                                        msg: "s.majuscule() ne prend aucun argument".into(),
                                        line: *line,
                                    });
                                }
                                Ok(Valeur::Str)
                            }
                            _ => Err(ErreurType {
                                msg: format!(
                                    "les string ont longueur() et majuscule(), pas « {} »",
                                    name
                                ),
                                line: *line,
                            }),
                        }
                    }
                    Valeur::Arr(_) => {
                        if name == "longueur" || name == "taille" {
                            if !args.is_empty() {
                                return Err(ErreurType {
                                    msg: "t.longueur() ne prend aucun argument".into(),
                                    line: *line,
                                });
                            }
                            Ok(Valeur::Int)
                        } else {
                            Err(ErreurType {
                                msg: format!("les tableaux ont longueur(), pas « {} »", name),
                                line: *line,
                            })
                        }
                    }
                    autre => Err(ErreurType {
                        msg: format!(
                            "on ne peut pas appeler « .{}() » sur « {} »",
                            name,
                            aff(&autre)
                        ),
                        line: *line,
                    }),
                }
            }
            Expr::Field { obj, name, line } => {
                let vo = self.expr(ctx, obj)?;
                match vo {
                    Valeur::Obj(ref classe) => {
                        if self.interfaces.contains_key(classe) {
                            return Err(ErreurType {
                                msg: format!(
                                    "« {} » est une interface : pas de champ accessible, uniquement ses méthodes",
                                    classe
                                ),
                                line: *line,
                            });
                        }
                        let c = match self.classes.get(classe).cloned() {
                            Some(c) => c,
                            None => {
                                return Err(ErreurType {
                                    msg: format!("le type « {} » n'existe pas", classe),
                                    line: *line,
                                })
                            }
                        };
                        match c.fields.iter().find(|f| f.name == *name) {
                            Some(f) => Ok(de_type(&f.ty)),
                            None => Err(ErreurType {
                                msg: format!(
                                    "la classe « {} » n'a pas de champ « {} »",
                                    classe, name
                                ),
                                line: *line,
                            }),
                        }
                    }
                    autre => Err(ErreurType {
                        msg: format!(
                            "on ne peut pas lire « .{} » sur « {} » (seuls les objets ont des champs)",
                            name,
                            aff(&autre)
                        ),
                        line: *line,
                    }),
                }
            }
            Expr::Index { obj, idx, line } => {
                let vo = self.expr(ctx, obj)?;
                let vi = self.expr(ctx, idx)?;
                self.doit_etre(&vi, &Valeur::Int, "l'indice d'un tableau doit être un int", *line);
                match vo {
                    Valeur::Arr(t) => Ok(*t),
                    autre => Err(ErreurType {
                        msg: format!(
                            "on ne peut indexer que des tableaux, trouvé « {} »",
                            aff(&autre)
                        ),
                        line: *line,
                    }),
                }
            }
            Expr::New { class, args, line } => {
                let c = self.classes.get(class).cloned();
                let c = match c {
                    Some(c) => c,
                    None => {
                        if self.interfaces.contains_key(class) {
                            return Err(ErreurType {
                                msg: format!(
                                    "« {} » est une interface : on n'instancie pas une interface — new une classe qui l'implémente",
                                    class
                                ),
                                line: *line,
                            });
                        }
                        return Err(ErreurType {
                            msg: format!(
                                "la classe « {} » n'existe pas (vérifie l'orthographe ou déclare-la)",
                                class
                            ),
                            line: *line,
                        })
                    }
                };
                let ctor = c.methods.iter().find(|m| m.est_constructeur(&c.name)).cloned();
                let ctor = match ctor {
                    Some(ctor) => ctor,
                    None => {
                        if args.is_empty() {
                            return Ok(Valeur::Obj(c.name));
                        }
                        return Err(ErreurType {
                            msg: format!(
                                "la classe « {} » n'a pas de constructeur fn {}(...) — et tu lui passes des arguments",
                                c.name, c.name
                            ),
                            line: *line,
                        });
                    }
                };
                self.verifier_signature(&ctor.params, &None, args, ctx, *line)?;
                Ok(Valeur::Obj(c.name))
            }
            Expr::ArrayLit { items, line } => {
                if items.is_empty() {
                    // type indéterminé : int[] par convention (rejeté si mal utilisé)
                    return Ok(Valeur::Arr(Box::new(Valeur::Int)));
                }
                let t0 = self.expr(ctx, &items[0])?;
                for it in items.iter().skip(1) {
                    let t = self.expr(ctx, it)?;
                    if t != t0 {
                        return Err(ErreurType {
                            msg: format!(
                                "tous les éléments d'un tableau doivent avoir le même type : {}",
                                attendu_trouve(&t0, &t)
                            ),
                            line: *line,
                        });
                    }
                }
                Ok(Valeur::Arr(Box::new(t0)))
            }
            Expr::Bin { op, l, r, line } => {
                let vl = self.expr(ctx, l)?;
                let vr = self.expr(ctx, r)?;
                use BinOp::*;
                match op {
                    Add => {
                        if vl == Valeur::Str && vr == Valeur::Str {
                            return Ok(Valeur::Str);
                        }
                        if vl == Valeur::Str || vr == Valeur::Str {
                            return Err(ErreurType {
                                msg: "la concaténation (+) demande deux string (ou convertis avec entier_vers_texte)".into(),
                                line: *line,
                            });
                        }
                        if (vl == Valeur::Int && vr == Valeur::Int)
                            || (vl == Valeur::Float && vr == Valeur::Float)
                        {
                            return Ok(vl);
                        }
                        Err(ErreurType {
                            msg: format!(
                                "l'addition (+) entre « {} » et « {} » n'existe pas (pas de mélange int/float)",
                                aff(&vl),
                                aff(&vr)
                            ),
                            line: *line,
                        })
                    }
                    Sub | Mul => {
                        if vl == vr && (vl == Valeur::Int || vl == Valeur::Float) {
                            Ok(vl)
                        } else {
                            Err(ErreurType {
                                msg: format!(
                                    "cette opération ({:?}) entre « {} » et « {} » n'existe pas",
                                    op,
                                    aff(&vl),
                                    aff(&vr)
                                ),
                                line: *line,
                            })
                        }
                    }
                    Div | Mod => {
                        if vl == vr && (vl == Valeur::Int || vl == Valeur::Float) {
                            Ok(vl)
                        } else {
                            Err(ErreurType {
                                msg: format!(
                                    "cette opération ({:?}) entre « {} » et « {} » n'existe pas",
                                    op,
                                    aff(&vl),
                                    aff(&vr)
                                ),
                                line: *line,
                            })
                        }
                    }
                    Eq | NotEq => {
                        if vl == vr && vl != Valeur::Void {
                            Ok(Valeur::Bool)
                        } else {
                            Err(ErreurType {
                                msg: format!(
                                    "comparer « {} » et « {} » avec == / != n'a pas de sens (types différents)",
                                    aff(&vl),
                                    aff(&vr)
                                ),
                                line: *line,
                            })
                        }
                    }
                    Lt | Le | Gt | Ge => {
                        if vl == vr && (vl == Valeur::Int || vl == Valeur::Float || vl == Valeur::Str) {
                            Ok(Valeur::Bool)
                        } else {
                            Err(ErreurType {
                                msg: format!(
                                    "comparer « {} » et « {} » avec < > <= >= n'existe pas",
                                    aff(&vl),
                                    aff(&vr)
                                ),
                                line: *line,
                            })
                        }
                    }
                    And | Or => {
                        if vl == Valeur::Bool && vr == Valeur::Bool {
                            Ok(Valeur::Bool)
                        } else {
                            Err(ErreurType {
                                msg: "&& et || demandent des bool des deux côtés".into(),
                                line: *line,
                            })
                        }
                    }
                }
            }
            Expr::Un { op, e: sub, line } => {
                let v = self.expr(ctx, sub)?;
                match op {
                    UnOp::Neg => {
                        if v == Valeur::Int || v == Valeur::Float {
                            Ok(v)
                        } else {
                            Err(ErreurType {
                                msg: format!("on ne peut pas négater « {} »", aff(&v)),
                                line: *line,
                            })
                        }
                    }
                    UnOp::Not => {
                        if v == Valeur::Bool {
                            Ok(Valeur::Bool)
                        } else {
                            Err(ErreurType {
                                msg: format!("« ! » demande un bool, trouvé « {} »", aff(&v)),
                                line: *line,
                            })
                        }
                    }
                }
            }
            // borrow est transparent : le compilateur emprunte déjà tout seul
            Expr::Borrow(inner, _) => self.expr(ctx, inner),
        }
    }

    fn verifier_signature(
        &mut self,
        params: &[Param],
        ret: &Option<Type>,
        args: &[Expr],
        ctx: &mut Ctx,
        line: usize,
    ) -> R<()> {
        if params.len() != args.len() {
            return Err(ErreurType {
                msg: format!(
                    "nombre d'arguments incorrect : la fonction demande {} argument(s), tu en donnes {}",
                    params.len(),
                    args.len()
                ),
                line,
            });
        }
        for (p, a) in params.iter().zip(args.iter()) {
            let va = self.expr(ctx, a)?;
            let vp = de_type(&p.ty);
            // passe par compatible() : un Chien EST un Animal (implements)
            if !self.compatible(&va, &vp) {
                return Err(ErreurType {
                    msg: format!(
                        "l'argument « {} » : {}",
                        p.name,
                        attendu_trouve(&vp, &va)
                    ),
                    line: a.ligne(),
                });
            }
        }
        let _ = ret;
        Ok(())
    }

    /// classe → interface : le polymorphisme du mariage
    fn compatible(&self, obtenu: &Valeur, attendu: &Valeur) -> bool {
        if obtenu == attendu {
            return true;
        }
        if let (Valeur::Obj(cl), Valeur::Obj(iface)) = (obtenu, attendu) {
            if let Some(cd) = self.classes.get(cl) {
                return cd.interfaces.iter().any(|i| i == iface);
            }
        }
        false
    }

    fn verifier_args_intrinseque_obj(
        &mut self,
        vo: &Valeur,
        _args: &[Expr],
        line: usize,
        _name: &String,
    ) -> R<()> {
        match vo {
            Valeur::Str | Valeur::Arr(_) => Ok(()),
            autre => Err(ErreurType {
                msg: format!("« longueur » n'existe que pour string et tableaux, pas « {} »", aff(autre)),
                line,
            }),
        }
    }

    fn appel(&mut self, ctx: &mut Ctx, callee: &str, args: &[Expr], line: usize) -> R<Valeur> {
        // 1. intrinsèque ?
        if let Some((n, ret)) = self.intrinseques.get(callee).cloned() {
            if args.len() != n {
                return Err(ErreurType {
                    msg: format!("« {} » prend {} argument(s), tu en donnes {}", callee, n, args.len()),
                    line,
                });
            }
            // vérifier les types des arguments des intrinsèques
            match callee {
                "print" | "afficher" => {
                    // accepte tout sauf void
                    for a in args {
                        let v = self.expr(ctx, a)?;
                        if v == Valeur::Void {
                            return Err(ErreurType {
                                msg: "afficher/print attend une valeur, pas un appel void".into(),
                                line: a.ligne(),
                            });
                        }
                    }
                }
                "longueur" => {
                    let v = self.expr(ctx, &args[0])?;
                    match v {
                        Valeur::Str | Valeur::Arr(_) => {}
                        autre => {
                            return Err(ErreurType {
                                msg: format!(
                                    "longueur attend une string ou un tableau, trouvé « {} »",
                                    aff(&autre)
                                ),
                                line,
                            })
                        }
                    }
                }
                "caractere" => {
                    let v0 = self.expr(ctx, &args[0])?;
                    let v1 = self.expr(ctx, &args[1])?;
                    if v0 != Valeur::Str {
                        return Err(ErreurType {
                            msg: "caractere(s, i) : le premier argument doit être une string".into(),
                            line,
                        });
                    }
                    if v1 != Valeur::Int {
                        return Err(ErreurType {
                            msg: "caractere(s, i) : le second argument doit être un int".into(),
                            line,
                        });
                    }
                }
                "entier_vers_texte" => {
                    let v = self.expr(ctx, &args[0])?;
                    if v != Valeur::Int {
                        return Err(ErreurType {
                            msg: "entier_vers_texte attend un int".into(),
                            line,
                        });
                    }
                }
                "texte_vers_entier" => {
                    let v = self.expr(ctx, &args[0])?;
                    if v != Valeur::Str {
                        return Err(ErreurType {
                            msg: "texte_vers_entier attend une string".into(),
                            line,
                        });
                    }
                }
                "dormir_ms" => {
                    let v = self.expr(ctx, &args[0])?;
                    if v != Valeur::Int {
                        return Err(ErreurType {
                            msg: "dormir_ms attend un int (millisecondes)".into(),
                            line,
                        });
                    }
                }
                "horloge_ms" => {}
                _ => {}
            }
            return Ok(ret.unwrap_or(Valeur::Void));
        }
        // 2. méthode de la classe courante ?
        if let Some(classe) = ctx.classe.clone() {
            let c = self.classes.get(&classe).cloned();
            if let Some(c) = c {
                if let Some(m) = c.methods.iter().find(|m| m.name == callee).cloned() {
                    self.verifier_signature(&m.params, &m.ret, args, ctx, line)?;
                    return Ok(de_type(m.ret.as_ref().unwrap_or(&Type::Void)));
                }
            }
        }
        // 3. fonction top-niveau ?
        if let Some(f) = self.fns.get(callee).cloned() {
            self.verifier_signature(&f.params, &f.ret, args, ctx, line)?;
            return Ok(de_type(f.ret.as_ref().unwrap_or(&Type::Void)));
        }
        Err(ErreurType {
            msg: format!("je ne connais pas la fonction « {} »", callee),
            line,
        })
    }
}
