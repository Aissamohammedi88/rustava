// ------------------------------------------------------------
// RUSTAVA — Parser : construit l'arbre syntaxique (AST)
// Grammaire : la forme de Java (accolades, ;, parenthèses optionnelles)
// ------------------------------------------------------------

use crate::ast::*;
use crate::lexer::{Span, Tok};

#[derive(Debug)]
pub struct ErreurParse {
    pub msg: String,
    pub line: usize,
}

type R<T> = Result<T, ErreurParse>;

pub struct Parser {
    toks: Vec<(Tok, Span)>,
    pos: usize,
}

fn nom_token(t: &Tok) -> String {
    match t {
        Tok::ClassK => "« class »".into(),
        Tok::InterfaceK => "« interface »".into(),
        Tok::ImplementsK => "« implements »".into(),
        Tok::ExtendsK => "« extends »".into(),
        Tok::FnK => "« fn »".into(),
        Tok::LetK => "« let »".into(),
        Tok::VarK => "« var »".into(),
        Tok::IntK => "« int »".into(),
        Tok::FloatK => "« float »".into(),
        Tok::BoolK => "« bool »".into(),
        Tok::StringK => "« string »".into(),
        Tok::IfK => "« if »".into(),
        Tok::ElseK => "« else »".into(),
        Tok::WhileK => "« while »".into(),
        Tok::ForK => "« for »".into(),
        Tok::InK => "« in »".into(),
        Tok::MatchK => "« match »".into(),
        Tok::TryK => "« try »".into(),
        Tok::CatchK => "« catch »".into(),
        Tok::ThrowK => "« throw »".into(),
        Tok::SpawnK => "« spawn »".into(),
        Tok::NewK => "« new »".into(),
        Tok::ReturnK => "« return »".into(),
        Tok::SelfK | Tok::ThisK => "« self »".into(),
        Tok::BorrowK => "« borrow »".into(),
        Tok::TrueK => "« true »".into(),
        Tok::FalseK => "« false »".into(),
        Tok::IntLit(_) => "un nombre entier".into(),
        Tok::FloatLit(_) => "un nombre flottant".into(),
        Tok::StrLit(_) => "une chaîne".into(),
        Tok::Ident(n) => format!("« {} »", n),
        Tok::Plus => "« + »".into(),
        Tok::Minus => "« - »".into(),
        Tok::Star => "« * »".into(),
        Tok::Slash => "« / »".into(),
        Tok::Percent => "« % »".into(),
        Tok::Assign => "« = »".into(),
        Tok::Eq => "« == »".into(),
        Tok::NotEq => "« != »".into(),
        Tok::Lt => "« < »".into(),
        Tok::Le => "« <= »".into(),
        Tok::Gt => "« > »".into(),
        Tok::Ge => "« >= »".into(),
        Tok::AndAnd => "« && »".into(),
        Tok::OrOr => "« || »".into(),
        Tok::Not => "« ! »".into(),
        Tok::PlusEq => "« += »".into(),
        Tok::MinusEq => "« -= »".into(),
        Tok::StarEq => "« *= »".into(),
        Tok::SlashEq => "« /= »".into(),
        Tok::LParen => "« ( »".into(),
        Tok::RParen => "« ) »".into(),
        Tok::LBrace => "« { »".into(),
        Tok::RBrace => "« } »".into(),
        Tok::LBracket => "« [ »".into(),
        Tok::RBracket => "« ] »".into(),
        Tok::Comma => "« , »".into(),
        Tok::Semi => "« ; »".into(),
        Tok::Colon => "« : »".into(),
        Tok::Arrow => "« -> »".into(),
        Tok::Dot => "« . »".into(),
        Tok::DotDot => "« .. »".into(),
        Tok::FatArrow => "« => »".into(),
        Tok::Eof => "la fin du fichier".into(),
    }
}

impl Parser {
    pub fn new(toks: Vec<(Tok, Span)>) -> Self {
        Parser { toks, pos: 0 }
    }

    fn ici(&self) -> &Tok {
        &self.toks[self.pos].0
    }
    fn ligne(&self) -> usize {
        self.toks[self.pos].1.line
    }
    fn avance(&mut self) -> (Tok, Span) {
        let t = self.toks[self.pos].clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }
    fn est(&self, t: &Tok) -> bool {
        self.toks[self.pos].0 == *t
    }
    fn est_eof(&self) -> bool {
        matches!(self.toks[self.pos].0, Tok::Eof)
    }
    fn mange(&mut self, t: &Tok) -> bool {
        if self.est(t) {
            self.avance();
            true
        } else {
            false
        }
    }
    fn err<T>(&self, msg: String) -> R<T> {
        Err(ErreurParse { msg, line: self.ligne() })
    }
    fn ident(&mut self, quoi: &str) -> R<(String, usize)> {
        if let Tok::Ident(n) = self.ici().clone() {
            let l = self.ligne();
            self.avance();
            Ok((n, l))
        } else {
            self.err(format!("j'attendais un nom de {} ; j'ai trouvé {}", quoi, nom_token(self.ici())))
        }
    }

    // ---------------- programme ----------------

    pub fn programme(&mut self) -> R<Program> {
        let mut items = Vec::new();
        loop {
            if self.est_eof() {
                break;
            }
            if self.est(&Tok::Semi) {
                self.avance();
                continue;
            }
            if self.est(&Tok::ClassK) {
                items.push(Item::Class(self.classe()?));
            } else if self.est(&Tok::InterfaceK) {
                items.push(Item::Interface(self.interface()?));
            } else if self.est(&Tok::FnK) {
                items.push(Item::Fn(self.fonction()?));
            } else {
                return self.err(format!(
                    "j'attendais « class », « interface » ou « fn » ; j'ai trouvé {}",
                    nom_token(self.ici())
                ));
            }
        }
        Ok(Program { items })
    }

    fn classe(&mut self) -> R<ClassDecl> {
        self.avance(); // class
        let (nom, ligne) = self.ident("classe")?;
        if nom.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false) {
            return self.err(format!(
                "une classe devrait commencer par une majuscule (« {} ») pour rester lisible",
                nom
            ));
        }
        let mut interfaces = Vec::new();
        if self.est(&Tok::ImplementsK) {
            self.avance();
            loop {
                let (i, _) = self.ident("interface")?;
                interfaces.push(i);
                if !self.mange(&Tok::Comma) {
                    break;
                }
            }
        }
        if self.est(&Tok::ExtendsK) {
            return self.err(
                "Rustava refuse l'héritage (« extends ») : compose avec des interfaces (implements) — c'est le mariage choisi"
                    .into(),
            );
        }
        self.attends(&Tok::LBrace)?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        while !self.est(&Tok::RBrace) {
            if self.est_eof() {
                return self.err("accolade « { » de la classe jamais fermée".into());
            }
            if self.est(&Tok::LetK) || self.est(&Tok::VarK) {
                let champ = self.champ()?;
                fields.push(champ);
            } else if self.est(&Tok::FnK) {
                let m = self.methode(&nom)?;
                methods.push(m);
            } else {
                return self.err(format!(
                    "dans une classe, j'attendais un champ (let/var) ou une méthode (fn) ; j'ai trouvé {}",
                    nom_token(self.ici())
                ));
            }
        }
        self.avance(); // }
        Ok(ClassDecl { name: nom, interfaces, fields, methods, line: ligne })
    }

    fn champ(&mut self) -> R<FieldDecl> {
        let mutable = if self.est(&Tok::LetK) {
            self.avance();
            false
        } else {
            self.avance();
            true
        };
        let (nom, ligne) = self.ident("champ")?;
        self.attends(&Tok::Colon)?;
        let ty = self.type_ann()?;
        if self.est(&Tok::Assign) {
            return self.err(format!(
                "le champ « {} » s'initialise dans le constructeur de la classe (une méthode fn portant le même nom), pas à sa déclaration",
                nom
            ));
        }
        self.attends(&Tok::Semi)?;
        Ok(FieldDecl { mutable, name: nom, ty, line: ligne })
    }

    fn methode(&mut self, _classe: &str) -> R<MethodDecl> {
        self.avance(); // fn
        let (nom, ligne) = self.ident("méthode")?;
        let (params, _) = self.params()?;
        let ret = self.ret_type()?;
        let corps = self.bloc()?;
        Ok(MethodDecl { name: nom, params, ret, body: corps, line: ligne })
    }

    fn interface(&mut self) -> R<InterfaceDecl> {
        self.avance();
        let (nom, ligne) = self.ident("interface")?;
        if nom.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false) {
            return self.err(format!(
                "une interface devrait commencer par une majuscule (« {} »)",
                nom
            ));
        }
        if self.est(&Tok::ExtendsK) {
            return self.err(
                "Rustava refuse l'héritage, même entre interfaces : implémente plusieurs interfaces plutôt".into(),
            );
        }
        self.attends(&Tok::LBrace)?;
        let mut methods = Vec::new();
        while !self.est(&Tok::RBrace) {
            if self.est_eof() {
                return self.err("accolade « { » de l'interface jamais fermée".into());
            }
            if self.est(&Tok::FnK) {
                self.avance();
                let (mn, ml) = self.ident("méthode d'interface")?;
                let (params, _) = self.params()?;
                let ret = self.ret_type()?;
                self.attends(&Tok::Semi)?;
                methods.push(MethSig { name: mn, params, ret, line: ml });
            } else {
                return self.err(format!(
                    "dans une interface, j'attendais une signature de méthode (fn ...) ; j'ai trouvé {}",
                    nom_token(self.ici())
                ));
            }
        }
        self.avance();
        Ok(InterfaceDecl { name: nom, methods, line: ligne })
    }

    fn fonction(&mut self) -> R<FnDecl> {
        self.avance(); // fn
        let (nom, ligne) = self.ident("fonction")?;
        let (params, _) = self.params()?;
        let ret = self.ret_type()?;
        let corps = self.bloc()?;
        Ok(FnDecl { name: nom, params, ret, body: corps, line: ligne })
    }

    fn params(&mut self) -> R<(Vec<Param>, usize)> {
        let ligne = self.ligne();
        self.attends(&Tok::LParen)?;
        let mut ps = Vec::new();
        if !self.est(&Tok::RParen) {
            loop {
                let (pn, pl) = self.ident("paramètre")?;
                self.attends(&Tok::Colon)?;
                let ty = self.type_ann()?;
                ps.push(Param { name: pn, ty, line: pl });
                if !self.mange(&Tok::Comma) {
                    break;
                }
            }
        }
        self.attends(&Tok::RParen)?;
        Ok((ps, ligne))
    }

    fn ret_type(&mut self) -> R<Option<Type>> {
        if self.est(&Tok::Arrow) {
            self.avance();
            Ok(Some(self.type_ann()?))
        } else {
            Ok(None)
        }
    }

    fn type_ann(&mut self) -> R<Type> {
        let base = if self.mange(&Tok::IntK) {
            Type::Int
        } else if self.mange(&Tok::FloatK) {
            Type::Float
        } else if self.mange(&Tok::BoolK) {
            Type::Bool
        } else if self.mange(&Tok::StringK) {
            Type::Str
        } else if matches!(self.ici(), Tok::Ident(_)) {
            let (n, _) = self.ident("type")?;
            // classe ou interface — tranché au typage
            Type::Class(n)
        } else {
            return self.err(format!(
                "j'attendais un type (int, float, bool, string, un nom de classe ou un tableau) ; j'ai trouvé {}",
                nom_token(self.ici())
            ));
        };
        let mut t = base;
        while self.est(&Tok::LBracket) {
            self.avance();
            self.attends(&Tok::RBracket)?;
            t = Type::Arr(Box::new(t));
        }
        Ok(t)
    }

    // ---------------- blocs et instructions ----------------

    fn bloc(&mut self) -> R<Block> {
        let ligne = self.ligne();
        self.attends(&Tok::LBrace)?;
        let mut stmts = Vec::new();
        loop {
            if self.est_eof() {
                return self.err("accolade « { » jamais fermée (fin du fichier atteinte)".into());
            }
            if self.est(&Tok::RBrace) {
                self.avance();
                break;
            }
            stmts.push(self.instruction()?);
        }
        Ok(Block { stmts, line: ligne })
    }

    fn instruction(&mut self) -> R<Stmt> {
        if self.mange(&Tok::Semi) {
            // instruction vide — on l'ignore avec élégance
            return Ok(Stmt::Block(Block { stmts: vec![], line: self.ligne() }, self.ligne()));
        }
        if self.est(&Tok::LBrace) {
            let b = self.bloc()?;
            let l = b.line;
            return Ok(Stmt::Block(b, l));
        }
        if self.est(&Tok::LetK) || self.est(&Tok::VarK) {
            let mutable = self.est(&Tok::VarK);
            self.avance();
            let (nom, ligne) = self.ident("variable")?;
            let ty = if self.mange(&Tok::Colon) {
                Some(self.type_ann()?)
            } else {
                None
            };
            self.attends(&Tok::Assign)?;
            let init = self.expr()?;
            self.attends(&Tok::Semi)?;
            return Ok(Stmt::Decl { mutable, name: nom, ty, init, line: ligne });
        }
        if self.est(&Tok::IfK) {
            return self.si();
        }
        if self.est(&Tok::WhileK) {
            self.avance();
            let cond = self.cond()?;
            let corps = self.bloc()?;
            let l = corps.line;
            return Ok(Stmt::While { cond, body: corps, line: l });
        }
        if self.est(&Tok::ForK) {
            self.avance();
            let (var, _) = self.ident("variable de boucle")?;
            self.attends(&Tok::InK)?;
            let debut = self.expr()?;
            let iter = if self.mange(&Tok::DotDot) {
                let fin = self.expr()?;
                Iter::Range(debut, fin)
            } else {
                Iter::Items(debut)
            };
            let corps = self.bloc()?;
            let l = corps.line;
            return Ok(Stmt::For { var, iter, body: corps, line: l });
        }
        if self.est(&Tok::MatchK) {
            return self.matchx();
        }
        if self.est(&Tok::TryK) {
            self.avance();
            let corps = self.bloc()?;
            self.attends(&Tok::CatchK)?;
            let (var, blk) = self.catch()?;
            let l = corps.line;
            return Ok(Stmt::Try { body: corps, catch_var: var, catch_blk: blk, line: l });
        }
        if self.est(&Tok::ThrowK) {
            self.avance();
            let e = self.expr()?;
            self.attends(&Tok::Semi)?;
            return Ok(Stmt::Throw(e, self.ligne()));
        }
        if self.est(&Tok::ReturnK) {
            self.avance();
            let val = if self.est(&Tok::Semi) {
                None
            } else {
                Some(self.expr()?)
            };
            self.attends(&Tok::Semi)?;
            return Ok(Stmt::Return(val, self.ligne()));
        }
        if self.est(&Tok::SpawnK) {
            self.avance();
            let e = self.expr()?;
            let l_spawn = e.ligne();
            if let Expr::Call { .. } = &e {
                self.attends(&Tok::Semi)?;
                return Ok(Stmt::Spawn(e, l_spawn));
            }
            if let Expr::MethodCall { .. } = &e {
                self.attends(&Tok::Semi)?;
                return Ok(Stmt::Spawn(e, l_spawn));
            }
            return self.err(format!(
                "spawn attend un appel de fonction, par exemple : spawn afficher(\"coucou\"); — j'ai trouvé autre chose (ligne {})",
                e.ligne()
            ));
        }
        // sinon : expression ou affectation
        let ligne = self.ligne();
        let e = self.expr()?;
        // affectation simple : cible = valeur
        if self.est(&Tok::Assign) {
            self.avance();
            self.verif_cible(&e)?;
            let val = self.expr()?;
            self.attends(&Tok::Semi)?;
            return Ok(Stmt::Assign { target: e, op: None, value: val, line: ligne });
        }
        // affectation composée : cible += -= *= /= valeur
        let op_composee = if self.est(&Tok::PlusEq) {
            Some(BinOp::Add)
        } else if self.est(&Tok::MinusEq) {
            Some(BinOp::Sub)
        } else if self.est(&Tok::StarEq) {
            Some(BinOp::Mul)
        } else if self.est(&Tok::SlashEq) {
            Some(BinOp::Div)
        } else {
            None
        };
        if let Some(op) = op_composee {
            self.avance();
            self.verif_cible(&e)?;
            let val = self.expr()?;
            self.attends(&Tok::Semi)?;
            return Ok(Stmt::Assign { target: e, op: Some(op), value: val, line: ligne });
        }
        self.attends(&Tok::Semi)?;
        Ok(Stmt::ExprStmt(e, ligne))
    }

    fn verif_cible(&self, e: &Expr) -> R<()> {
        match e {
            Expr::Ident(..) | Expr::Field { .. } | Expr::Index { .. } => Ok(()),
            _ => self.err("cette expression ne peut pas recevoir une valeur (=) : il faut une variable, un champ ou un élément de tableau".into()),
        }
    }

    fn cond(&mut self) -> R<Expr> {
        if self.mange(&Tok::LParen) {
            let e = self.expr()?;
            self.attends(&Tok::RParen)?;
            Ok(e)
        } else {
            self.expr()
        }
    }

    fn si(&mut self) -> R<Stmt> {
        self.avance(); // if
        let ligne = self.ligne();
        let mut arms = Vec::new();
        let cond = self.cond()?;
        let corps = self.bloc()?;
        arms.push((cond, corps));
        let mut else_blk = None;
        while self.est(&Tok::ElseK) {
            self.avance();
            if self.est(&Tok::IfK) {
                self.avance();
                let c = self.cond()?;
                let b = self.bloc()?;
                arms.push((c, b));
            } else {
                else_blk = Some(self.bloc()?);
                break;
            }
        }
        Ok(Stmt::If { arms, else_blk, line: ligne })
    }

    fn matchx(&mut self) -> R<Stmt> {
        self.avance();
        let ligne = self.ligne();
        let sujet = self.cond()?;
        self.attends(&Tok::LBrace)?;
        let mut arms = Vec::new();
        while !self.est(&Tok::RBrace) {
            if self.est_eof() {
                return self.err("accolade « { » du match jamais fermée".into());
            }
            let (pat, al) = self.motif()?;
            self.attends(&Tok::FatArrow)?;
            let corps = if self.est(&Tok::LBrace) {
                self.bloc()?
            } else {
                // une seule instruction : 1 => print("un"),
                let l = self.ligne();
                let e = self.expr()?;
                if self.mange(&Tok::Semi) || self.mange(&Tok::Comma) {
                    // ok
                } else {
                    return self.err("après la branche du match, j'attendais « ; », « , » ou un bloc « { ... } »".into());
                }
                Block { stmts: vec![Stmt::ExprStmt(e, l)], line: l }
            };
            if self.est(&Tok::Comma) {
                self.avance();
            }
            arms.push(MatchArm { pat, body: corps, line: al });
        }
        self.avance(); // }
        Ok(Stmt::Match { subject: sujet, arms, line: ligne })
    }

    fn motif(&mut self) -> R<(MatchPat, usize)> {
        let l = self.ligne();
        match self.ici().clone() {
            Tok::IntLit(n) => {
                self.avance();
                Ok((MatchPat::IntLit(n), l))
            }
            Tok::FloatLit(f) => {
                self.avance();
                Ok((MatchPat::FloatLit(f), l))
            }
            Tok::StrLit(s) => {
                self.avance();
                Ok((MatchPat::StrLit(s), l))
            }
            Tok::TrueK => {
                self.avance();
                Ok((MatchPat::BoolLit(true), l))
            }
            Tok::FalseK => {
                self.avance();
                Ok((MatchPat::BoolLit(false), l))
            }
            Tok::Ident(n) if n == "_" => {
                self.avance();
                Ok((MatchPat::Wildcard, l))
            }
            autre => self.err(format!(
                "un motif de match doit être un littéral (nombre, chaîne, true/false) ou « _ » ; j'ai trouvé {}",
                nom_token(&autre)
            )),
        }
    }

    fn catch(&mut self) -> R<(Option<String>, Block)> {
        let var = if self.mange(&Tok::LParen) {
            let (n, _) = self.ident("variable d'erreur")?;
            self.attends(&Tok::RParen)?;
            Some(n)
        } else {
            None
        };
        let blk = self.bloc()?;
        Ok((var, blk))
    }

    // ---------------- expressions ----------------

    pub fn expr(&mut self) -> R<Expr> {
        self.ou()
    }

    fn ou(&mut self) -> R<Expr> {
        let mut g = self.et()?;
        while self.est(&Tok::OrOr) {
            self.avance();
            let d = self.et()?;
            let l = g.ligne();
            g = Expr::Bin { op: BinOp::Or, l: Box::new(g), r: Box::new(d), line: l };
        }
        Ok(g)
    }

    fn et(&mut self) -> R<Expr> {
        let mut g = self.egal()?;
        while self.est(&Tok::AndAnd) {
            self.avance();
            let d = self.egal()?;
            let l = g.ligne();
            g = Expr::Bin { op: BinOp::And, l: Box::new(g), r: Box::new(d), line: l };
        }
        Ok(g)
    }

    fn egal(&mut self) -> R<Expr> {
        let mut g = self.relatif()?;
        loop {
            let op = if self.est(&Tok::Eq) {
                BinOp::Eq
            } else if self.est(&Tok::NotEq) {
                BinOp::NotEq
            } else {
                break;
            };
            self.avance();
            let d = self.relatif()?;
            let l = g.ligne();
            g = Expr::Bin { op, l: Box::new(g), r: Box::new(d), line: l };
        }
        Ok(g)
    }

    fn relatif(&mut self) -> R<Expr> {
        let mut g = self.addition()?;
        loop {
            let op = if self.est(&Tok::Lt) {
                BinOp::Lt
            } else if self.est(&Tok::Le) {
                BinOp::Le
            } else if self.est(&Tok::Gt) {
                BinOp::Gt
            } else if self.est(&Tok::Ge) {
                BinOp::Ge
            } else {
                break;
            };
            self.avance();
            let d = self.addition()?;
            let l = g.ligne();
            g = Expr::Bin { op, l: Box::new(g), r: Box::new(d), line: l };
        }
        Ok(g)
    }

    fn addition(&mut self) -> R<Expr> {
        let mut g = self.multiplication()?;
        loop {
            let op = if self.est(&Tok::Plus) {
                BinOp::Add
            } else if self.est(&Tok::Minus) {
                BinOp::Sub
            } else {
                break;
            };
            self.avance();
            let d = self.multiplication()?;
            let l = g.ligne();
            g = Expr::Bin { op, l: Box::new(g), r: Box::new(d), line: l };
        }
        Ok(g)
    }

    fn multiplication(&mut self) -> R<Expr> {
        let mut g = self.unaire()?;
        loop {
            let op = if self.est(&Tok::Star) {
                BinOp::Mul
            } else if self.est(&Tok::Slash) {
                BinOp::Div
            } else if self.est(&Tok::Percent) {
                BinOp::Mod
            } else {
                break;
            };
            self.avance();
            let d = self.unaire()?;
            let l = g.ligne();
            g = Expr::Bin { op, l: Box::new(g), r: Box::new(d), line: l };
        }
        Ok(g)
    }

    fn unaire(&mut self) -> R<Expr> {
        if self.est(&Tok::Minus) {
            self.avance();
            let e = self.unaire()?;
            let l = e.ligne();
            Ok(Expr::Un { op: UnOp::Neg, e: Box::new(e), line: l })
        } else if self.est(&Tok::Not) {
            self.avance();
            let e = self.unaire()?;
            let l = e.ligne();
            Ok(Expr::Un { op: UnOp::Not, e: Box::new(e), line: l })
        } else if self.est(&Tok::BorrowK) {
            self.avance();
            let e = self.unaire()?;
            let l = e.ligne();
            Ok(Expr::Borrow(Box::new(e), l))
        } else {
            self.postfixe()
        }
    }

    fn postfixe(&mut self) -> R<Expr> {
        let mut e = self.primaire()?;
        loop {
            if self.est(&Tok::Dot) {
                self.avance();
                let (nom, l) = self.ident("champ ou méthode")?;
                if self.est(&Tok::LParen) {
                    let args = self.args()?;
                    e = Expr::MethodCall { obj: Box::new(e), name: nom, args, line: l };
                } else {
                    e = Expr::Field { obj: Box::new(e), name: nom, line: l };
                }
            } else if self.est(&Tok::LBracket) {
                self.avance();
                let idx = self.expr()?;
                self.attends(&Tok::RBracket)?;
                let l = e.ligne();
                e = Expr::Index { obj: Box::new(e), idx: Box::new(idx), line: l };
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn args(&mut self) -> R<Vec<Expr>> {
        self.attends(&Tok::LParen)?;
        let mut as_ = Vec::new();
        if !self.est(&Tok::RParen) {
            loop {
                as_.push(self.expr()?);
                if !self.mange(&Tok::Comma) {
                    break;
                }
            }
        }
        self.attends(&Tok::RParen)?;
        Ok(as_)
    }

    fn primaire(&mut self) -> R<Expr> {
        let l = self.ligne();
        match self.ici().clone() {
            Tok::IntLit(n) => {
                self.avance();
                Ok(Expr::IntLit(n, l))
            }
            Tok::FloatLit(f) => {
                self.avance();
                Ok(Expr::FloatLit(f, l))
            }
            Tok::StrLit(s) => {
                self.avance();
                Ok(Expr::StrLit(s, l))
            }
            Tok::TrueK => {
                self.avance();
                Ok(Expr::BoolLit(true, l))
            }
            Tok::FalseK => {
                self.avance();
                Ok(Expr::BoolLit(false, l))
            }
            Tok::SelfK => {
                self.avance();
                Ok(Expr::Ident("self".into(), l))
            }
            Tok::ThisK => {
                // Java et Rust réconciliés : this == self
                self.avance();
                Ok(Expr::Ident("self".into(), l))
            }
            Tok::LParen => {
                self.avance();
                let e = self.expr()?;
                self.attends(&Tok::RParen)?;
                Ok(e)
            }
            Tok::NewK => {
                self.avance();
                let (cls, cl) = self.ident("classe")?;
                let args = if self.est(&Tok::LParen) {
                    self.args()?
                } else {
                    vec![]
                };
                Ok(Expr::New { class: cls, args, line: cl })
            }
            Tok::LBracket => {
                self.avance();
                let mut items = Vec::new();
                if !self.est(&Tok::RBracket) {
                    loop {
                        items.push(self.expr()?);
                        if !self.mange(&Tok::Comma) {
                            break;
                        }
                    }
                }
                self.attends(&Tok::RBracket)?;
                Ok(Expr::ArrayLit { items, line: l })
            }
            Tok::Ident(n) => {
                self.avance();
                if self.est(&Tok::LParen) {
                    let args = self.args()?;
                    Ok(Expr::Call { callee: n, args, line: l })
                } else {
                    Ok(Expr::Ident(n, l))
                }
            }
            t => self.err(format!(
                "j'attendais une expression (nombre, chaîne, variable, appel...) ; j'ai trouvé {}",
                nom_token(&t)
            )),
        }
    }

    // petites aides (utilisées par le checker via le même flux)
    fn attends(&mut self, t: &Tok) -> R<()> {
        if self.est(t) {
            self.avance();
            Ok(())
        } else {
            self.err(format!(
                "j'ai trouvé {}, j'attendais {}",
                nom_token(self.ici()),
                nom_token(t)
            ))
        }
    }
}
