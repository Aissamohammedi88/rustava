// ------------------------------------------------------------
// RUSTAVA — Lexer : découpe le code en tokens (mots, symboles)
// ------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    // mots-clés (le mariage Java × Rust)
    ClassK,
    InterfaceK,
    ImplementsK,
    ExtendsK, // reconnu SEULEMENT pour refuser avec un message gentil
    FnK,
    LetK,
    VarK,
    IntK,
    FloatK,
    BoolK,
    StringK,
    IfK,
    ElseK,
    WhileK,
    ForK,
    InK,
    MatchK,
    TryK,
    CatchK,
    ThrowK,
    SpawnK,
    NewK,
    ReturnK,
    SelfK,
    ThisK,
    BorrowK,
    TrueK,
    FalseK,
    // littéraux et noms
    IntLit(i64),
    FloatLit(f64),
    StrLit(String),
    Ident(String),
    // symboles
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign, // =
    Eq,     // ==
    NotEq,  // !=
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Not,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Colon,
    Arrow,   // ->
    Dot,     // .
    DotDot,  // ..
    FatArrow, // =>
    Eof,
}

#[derive(Clone, Debug)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug)]
pub struct ErreurLex {
    pub msg: String,
    pub line: usize,
    pub col: usize,
}

pub fn lexer(src: &str) -> Result<Vec<(Tok, Span)>, ErreurLex> {
    let mut toks = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    macro_rules! pousse {
        ($t:expr) => {
            toks.push(($t, Span { line, col }))
        };
    }

    while i < chars.len() {
        let c = chars[i];

        // blancs
        if c == '\n' {
            i += 1;
            line += 1;
            col = 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            col += 1;
            continue;
        }

        // commentaire //
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        // commentaire /* */
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            let (dl, dc) = (line, col);
            i += 2;
            col += 2;
            let mut profondeur = 1;
            while i < chars.len() && profondeur > 0 {
                if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                    profondeur += 1;
                    i += 2;
                    col += 2;
                } else if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    profondeur -= 1;
                    i += 2;
                    col += 2;
                } else if chars[i] == '\n' {
                    i += 1;
                    line += 1;
                    col = 1;
                } else {
                    i += 1;
                    col += 1;
                }
            }
            if profondeur > 0 {
                return Err(ErreurLex {
                    msg: "commentaire /* ouvert mais jamais fermé".into(),
                    line: dl,
                    col: dc,
                });
            }
            continue;
        }

        // chaîne "..."
        if c == '"' {
            let (dl, dc) = (line, col);
            i += 1;
            col += 1;
            let mut s = String::new();
            loop {
                if i >= chars.len() {
                    return Err(ErreurLex {
                        msg: "guillemet ouvert mais jamais fermé".into(),
                        line: dl,
                        col: dc,
                    });
                }
                let ch = chars[i];
                if ch == '"' {
                    i += 1;
                    col += 1;
                    break;
                }
                if ch == '\\' {
                    if i + 1 >= chars.len() {
                        return Err(ErreurLex {
                            msg: "échappement \\ en fin de fichier".into(),
                            line,
                            col,
                        });
                    }
                    let n = chars[i + 1];
                    let ech = match n {
                        'n' => '\n',
                        't' => '\t',
                        '\\' => '\\',
                        '"' => '"',
                        '0' => '\0',
                        _ => {
                            return Err(ErreurLex {
                                msg: format!("échappement inconnu \\{}", n),
                                line,
                                col,
                            })
                        }
                    };
                    s.push(ech);
                    i += 2;
                    col += 2;
                    continue;
                }
                if ch == '\n' {
                    return Err(ErreurLex {
                        msg: "retour à la ligne dans une chaîne (ferme-la avec \")".into(),
                        line,
                        col,
                    });
                }
                s.push(ch);
                i += 1;
                col += 1;
            }
            pousse!(Tok::StrLit(s));
            continue;
        }

        // nombres
        if c.is_ascii_digit() {
            let (dl, dc) = (line, col);
            let mut n = String::new();
            let mut flottant = false;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_') {
                if chars[i] != '_' {
                    n.push(chars[i]);
                }
                i += 1;
                col += 1;
            }
            if i < chars.len() && chars[i] == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()
            {
                flottant = true;
                n.push('.');
                i += 1;
                col += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    n.push(chars[i]);
                    i += 1;
                    col += 1;
                }
            }
            if i < chars.len() && (chars[i].is_alphabetic() || chars[i] == '_') {
                return Err(ErreurLex {
                    msg: format!("nombre suivi d'une lettre : « {}{} »", n, chars[i]),
                    line: dl,
                    col: dc,
                });
            }
            if flottant {
                pousse!(Tok::FloatLit(n.parse::<f64>().map_err(|_| ErreurLex {
                    msg: "nombre flottant invalide".into(),
                    line: dl,
                    col: dc,
                })?));
            } else {
                pousse!(Tok::IntLit(n.parse::<i64>().map_err(|_| ErreurLex {
                    msg: "nombre entier trop grand (dépasse 64 bits)".into(),
                    line: dl,
                    col: dc,
                })?));
            }
            continue;
        }

        // identifiants et mots-clés
        if c.is_ascii_alphabetic() || c == '_' {
            let mut n = String::new();
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                n.push(chars[i]);
                i += 1;
                col += 1;
            }
            let t = match n.as_str() {
                "class" => Tok::ClassK,
                "interface" => Tok::InterfaceK,
                "implements" => Tok::ImplementsK,
                "extends" => Tok::ExtendsK,
                "fn" => Tok::FnK,
                "let" => Tok::LetK,
                "var" => Tok::VarK,
                "int" => Tok::IntK,
                "float" => Tok::FloatK,
                "bool" => Tok::BoolK,
                "string" => Tok::StringK,
                "if" => Tok::IfK,
                "else" => Tok::ElseK,
                "while" => Tok::WhileK,
                "for" => Tok::ForK,
                "in" => Tok::InK,
                "match" => Tok::MatchK,
                "try" => Tok::TryK,
                "catch" => Tok::CatchK,
                "throw" => Tok::ThrowK,
                "spawn" => Tok::SpawnK,
                "new" => Tok::NewK,
                "return" => Tok::ReturnK,
                "self" => Tok::SelfK,
                "this" => Tok::ThisK,
                "borrow" => Tok::BorrowK,
                "true" => Tok::TrueK,
                "false" => Tok::FalseK,
                _ => Tok::Ident(n),
            };
            pousse!(t);
            continue;
        }

        // symboles (les longs d'abord)
        let deux = |a: char, b: char| i + 1 < chars.len() && chars[i] == a && chars[i + 1] == b;
        let trois = |a: char, b: char, d: char| {
            i + 2 < chars.len() && chars[i] == a && chars[i + 1] == b && chars[i + 2] == d
        };
        let t = if trois('.', '.', '.') {
            return Err(ErreurLex { msg: "« ... » n'existe pas en Rustava (utilise a..b)".into(), line, col });
        } else if deux('=', '=') {
            i += 2;
            col += 2;
            Tok::Eq
        } else if deux('!', '=') {
            i += 2;
            col += 2;
            Tok::NotEq
        } else if deux('<', '=') {
            i += 2;
            col += 2;
            Tok::Le
        } else if deux('>', '=') {
            i += 2;
            col += 2;
            Tok::Ge
        } else if deux('&', '&') {
            i += 2;
            col += 2;
            Tok::AndAnd
        } else if deux('|', '|') {
            i += 2;
            col += 2;
            Tok::OrOr
        } else if deux('+', '=') {
            i += 2;
            col += 2;
            Tok::PlusEq
        } else if deux('-', '=') {
            i += 2;
            col += 2;
            Tok::MinusEq
        } else if deux('*', '=') {
            i += 2;
            col += 2;
            Tok::StarEq
        } else if deux('/', '=') {
            i += 2;
            col += 2;
            Tok::SlashEq
        } else if deux('-', '>') {
            i += 2;
            col += 2;
            Tok::Arrow
        } else if deux('=', '>') {
            i += 2;
            col += 2;
            Tok::FatArrow
        } else if deux('.', '.') {
            i += 2;
            col += 2;
            Tok::DotDot
        } else {
            let simple = match c {
                '+' => Tok::Plus,
                '-' => Tok::Minus,
                '*' => Tok::Star,
                '/' => Tok::Slash,
                '%' => Tok::Percent,
                '=' => Tok::Assign,
                '<' => Tok::Lt,
                '>' => Tok::Gt,
                '!' => Tok::Not,
                '(' => Tok::LParen,
                ')' => Tok::RParen,
                '{' => Tok::LBrace,
                '}' => Tok::RBrace,
                '[' => Tok::LBracket,
                ']' => Tok::RBracket,
                ',' => Tok::Comma,
                ';' => Tok::Semi,
                ':' => Tok::Colon,
                '.' => Tok::Dot,
                _ => {
                    return Err(ErreurLex {
                        msg: format!("caractère inattendu « {} »", c),
                        line,
                        col,
                    })
                }
            };
            i += 1;
            col += 1;
            simple
        };
        pousse!(t);
    }
    toks.push((Tok::Eof, Span { line, col }));
    Ok(toks)
}
