use std::any::Any;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fs;
use std::path::Path;

use lex_just_parse::lexer::*;
use lex_just_parse::parser::{Parser, RefLexer, many1, sep_by};
use lex_just_parse::try_parse;

pub type Value = Box<dyn Any + 'static>;
pub type Percent = f32;
pub type Gss = Object;

#[derive(Debug)]
pub struct Object {
    inner: HashMap<String, Value>,
    max_depth: usize,
}

#[derive(Debug)]
pub enum Expr {
    Symbol(String),
    Access(Vec<String>),
    RelAccess(Vec<String>),
}

impl Object {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            max_depth: 20,
        }
    }

    pub fn dump(&self, level: usize) {
        println!("{{");

        for (k, v) in self.inner.iter() {
            for _ in 0..=level {
                print!("    ");
            }

            print!("{k} => ");

            if let Some(string) = v.downcast_ref::<String>() {
                println!("\"{}\"", string);
            } else if let Some(i) = v.downcast_ref::<i32>() {
                println!("{}", i);
            } else if let Some(i) = v.downcast_ref::<f32>() {
                println!("{}", i);
            } else if let Some(b) = v.downcast_ref::<bool>() {
                println!("{}", b);
            } else if let Some(obj) = v.downcast_ref::<Object>() {
                obj.dump(level + 1);
            } else if let Some(expr) = v.downcast_ref::<Expr>() {
                match expr {
                    Expr::Symbol(s) => println!("{s}"),
                    Expr::Access(seq) => {
                        for (i, s) in seq.iter().enumerate() {
                            if i > 0 {
                                print!(".");
                            }
                            print!("{s}");
                        }
                        println!()
                    }
                    Expr::RelAccess(seq) => {
                        print!(".");
                        for (i, s) in seq.iter().enumerate() {
                            if i > 0 {
                                print!(".");
                            }
                            print!("{s}");
                        }
                        println!()
                    }
                }
            } else {
                println!("UNKNOWN({:?})", v.type_id());
            }
        }

        for _ in 0..level {
            print!("    ");
        }

        println!("}}");
    }

    pub fn get_fields(&self) -> std::collections::hash_map::Keys<'_, String, Value> {
        self.inner.keys()
    }

    pub fn get<T: 'static>(&self, path: &[&str]) -> Option<&T> {
        self.get_impl(path, 0, self.max_depth)
    }

    pub fn get_or_default<T: Clone + 'static + Default>(&self, path: &[&str]) -> T {
        self.get_impl(path, 0, self.max_depth)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_or<T: Clone + 'static>(&self, path: &[&str], default: T) -> T {
        self.get_impl(path, 0, self.max_depth)
            .cloned()
            .unwrap_or(default)
    }

    fn get_impl<T: 'static>(
        &self,
        path: &[&str],
        current_depth: usize,
        max_depth: usize,
    ) -> Option<&T> {
        if current_depth >= max_depth {
            return None;
        }
        let mut obj = self;
        if let Some((last, prefix)) = path.split_last() {
            for c in prefix {
                if let Some(v) = obj.inner.get(*c) {
                    if let Some(o) = v.downcast_ref::<Object>() {
                        obj = o;
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }

            if let Some(v) = obj.inner.get(*last) {
                if let Some(expr) = v.downcast_ref::<Expr>() {
                    return match expr {
                        Expr::Symbol(s) => {
                            self.get_impl(&[s.as_str()], current_depth + 1, max_depth)
                        }
                        Expr::Access(seq) => {
                            let tmp: Vec<&str> = seq.iter().map(AsRef::as_ref).collect();
                            self.get_impl(&tmp, current_depth + 1, max_depth)
                        }
                        Expr::RelAccess(seq) => {
                            let tmp: Vec<&str> = seq.iter().map(AsRef::as_ref).collect();
                            obj.get_impl(&tmp, current_depth + 1, max_depth)
                        }
                    };
                }
                return v.downcast_ref::<T>();
            }
        }
        None
    }

    /// Default = 20
    pub fn set_max_depth(&mut self, max_depth: usize) {
        self.max_depth = max_depth;
    }
}

pub fn load_gss_from_file<P: AsRef<Path>>(file_path: P) -> Result<Gss, Box<dyn StdError>> {
    let source = fs::read_to_string(file_path.as_ref())?;
    let mut lex = Lexer::new(&source);

    let gss = parse(file_path, &mut lex)?;

    Ok(gss)
}

fn parse<'lex, P: AsRef<Path>>(
    file_path: P,
    lex: RefLexer<'lex>,
) -> Result<Gss, Box<dyn StdError>> {
    match parse_gss(lex) {
        Parser::Success(_, object) => Ok(object),
        Parser::Fail(lex, err) => Err(format!(
            "{}:{}: {}",
            file_path.as_ref().display(),
            lex.peek().loc,
            err
        )
        .into()),
    }
}

fn parse_gss<'lex>(mut lex: RefLexer) -> Parser<Gss, Box<dyn StdError>> {
    let mut object = Object::new();
    if lex.peek().kind == TokenKind::EOF {
        return Parser::Success(lex, object);
    }
    let fields = try_parse!(
        lex,
        many1(lex, |mut lex| {
            let k = try_parse!(lex, parse_ident(lex));
            try_parse!(lex, parse_eq(lex));
            let v = try_parse!(lex, parse_value(lex));
            if lex.peek().kind == TokenKind::Comma {
                lex.next();
            }
            Parser::Success(lex, (k, v))
        })
    );
    try_parse!(lex, parse_eof(lex));
    for (key, value) in fields {
        if object.inner.insert(key.to_string(), value).is_some() {
            return Parser::Fail(lex, format!("Redefinition of key {key}").into());
        }
    }
    Parser::Success(lex, object)
}

fn parse_object<'lex>(mut lex: RefLexer) -> Parser<Object, Box<dyn StdError>> {
    let mut object = Object::new();
    if lex.peek().kind == TokenKind::CloseCurly {
        lex.next();
        return Parser::Success(lex, object);
    }
    let fields = try_parse!(
        lex,
        sep_by(
            lex,
            |mut lex| {
                let k = try_parse!(lex, parse_ident(lex));
                try_parse!(lex, parse_eq(lex));
                let v = try_parse!(lex, parse_value(lex));
                Parser::Success(lex, (k, v))
            },
            parse_comma
        )
    );
    try_parse!(lex, parse_close_curly(lex));
    for (key, value) in fields {
        if object.inner.insert(key.to_string(), value).is_some() {
            return Parser::Fail(lex, format!("Redefinition of key {key}").into());
        }
    }
    Parser::Success(lex, object)
}

fn parse_value<'lex>(mut lex: RefLexer) -> Parser<Value, Box<dyn StdError>> {
    let t = lex.next();
    match t.kind {
        TokenKind::Number(base) => {
            let x = match i32::from_str_radix(t.source(), base.radix()) {
                Ok(x) => x,
                Err(err) => return Parser::Fail(lex, err.into()),
            };
            if lex.peek().kind == TokenKind::Mod {
                lex.next();
                return Parser::Success(lex, Box::new(x as f32 / 100.));
            }
            Parser::Success(lex, Box::new(x))
        }
        TokenKind::RealNumber => {
            let x = match t.source().parse::<f32>() {
                Ok(x) => x,
                Err(err) => return Parser::Fail(lex, err.into()),
            };
            if lex.peek().kind == TokenKind::Mod {
                lex.next();
                return Parser::Success(lex, Box::new(x as f32 / 100.));
            }
            Parser::Success(lex, Box::new(x))
        }
        TokenKind::Identifier if t.source() == "true" => Parser::Success(lex, Box::new(true)),
        TokenKind::Identifier if t.source() == "false" => Parser::Success(lex, Box::new(false)),
        TokenKind::StringLiteral => Parser::Success(lex, Box::new(t.unescape())),
        TokenKind::OpenCurly => {
            let object = try_parse!(lex, parse_object(lex));
            Parser::Success(lex, Box::new(object))
        }
        TokenKind::Identifier => {
            if lex.peek().kind == TokenKind::Dot {
                lex.next();
                let mut seq = vec![t.source.to_string()];
                seq.extend(
                    try_parse!(lex, sep_by(lex, parse_ident, parse_dot))
                        .into_iter()
                        .map(|t| t.source.to_string()),
                );
                return Parser::Success(lex, Box::new(Expr::Access(seq)));
            }
            Parser::Success(lex, Box::new(Expr::Symbol(t.source.to_string())))
        }
        TokenKind::Dot => {
            if lex.peek().kind == TokenKind::Identifier {
                let t = lex.next();
                let mut seq = vec![t.source.to_string()];
                seq.extend(
                    try_parse!(lex, sep_by(lex, parse_ident, parse_dot))
                        .into_iter()
                        .map(|t| t.source.to_string()),
                );
                return Parser::Success(lex, Box::new(Expr::RelAccess(seq)));
            }
            Parser::Fail(lex, format!("Unexpect token `{t}`").into())
        }
        _ => Parser::Fail(lex, format!("Unexpect token `{t}`").into()),
    }
}

macro_rules! make_expect {
    ($name:ident, $kind:expr, $repr:literal) => {
        fn $name<'lex>(lex: RefLexer) -> Parser<(), Box<dyn StdError>> {
            let actual = lex.peek().kind;
            if actual != $kind {
                return Parser::Fail(lex, format!("Expect {} got {actual:?}", $repr).into());
            }
            lex.next();
            Parser::Success(lex, ())
        }
    };
    (ret, $name:ident, $kind:expr, $repr:literal) => {
        fn $name<'lex>(lex: RefLexer) -> Parser<Token, Box<dyn StdError>> {
            let actual = lex.peek().kind;
            if actual != $kind {
                return Parser::Fail(lex, format!("Expect {} got {actual:?}", $repr).into());
            }
            let t = lex.next();
            Parser::Success(lex, t)
        }
    };
}

make_expect! {parse_dot, TokenKind::Dot, "." }
make_expect! {parse_comma, TokenKind::Comma, "," }
make_expect! {parse_eq, TokenKind::Eq, "=" }
make_expect! {parse_close_curly, TokenKind::CloseCurly, "}" }
make_expect! {parse_eof, TokenKind::EOF, "EOF" }
make_expect! {ret, parse_ident, TokenKind::Identifier, "identifier" }

#[cfg(test)]
mod tests {
    use super::*;
    use lex_just_parse::lexer::Lexer;

    fn parse_str(source: &str) -> Result<Gss, Box<dyn std::error::Error>> {
        let mut lex = Lexer::new(source);
        parse("test_string", &mut lex)
    }

    #[test]
    fn test_parse_success() {
        let source = r#"
            name = "GSS",
            version = 1,
            active = true,
            settings = {
                theme = "dark",
                debug = false,
            },
        "#;
        let gss = parse_str(source).expect("Should parse successfully");

        // Test basic values
        assert_eq!(gss.get::<String>(&["name"]), Some(&"GSS".to_string()));
        assert_eq!(gss.get::<i32>(&["version"]), Some(&1));
        assert_eq!(gss.get::<bool>(&["active"]), Some(&true));

        // Test nested values
        assert_eq!(
            gss.get::<String>(&["settings", "theme"]),
            Some(&"dark".to_string())
        );
        assert_eq!(gss.get::<bool>(&["settings", "debug"]), Some(&false));

        // Test non-existent keys / incorrect types
        assert_eq!(gss.get::<String>(&["non_existent"]), None);
        assert_eq!(gss.get::<String>(&["settings", "non_existent"]), None);
        assert_eq!(gss.get::<i32>(&["active"]), None); // Type mismatch
    }

    #[test]
    fn test_parse_redefinition() {
        let source = r#"
            key = 1,
            key = 2,
        "#;
        let result = parse_str(source);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Redefinition of key key"));
    }

    #[test]
    fn test_parse_missing_comma() {
        let source = r#"
            test = {
                key = 1
                other = 2,
            }
        "#;
        let result = parse_str(source);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_eq() {
        let source = r#"
            key 1,
        "#;
        let result = parse_str(source);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_path() {
        let gss = parse_str("a = 1,").expect("Should parse");
        assert_eq!(gss.get::<i32>(&[]), None);
    }

    #[test]
    fn test_dump() {
        let source = r#"
            name = "GSS",
            version = 1,
            active = true,
            settings = {
                theme = "dark",
            },
        "#;
        let gss = parse_str(source).expect("Should parse");
        // Ensure dump runs without panicking
        gss.dump(0);
    }

    #[test]
    fn test_references() {
        let source = r#"
            root_val = 42,
            ref_symbol = root_val,
            nested = {
                value = 100,
                ref_symbol_nested = root_val,
            },
            ref_access = nested.value,
            other = {
                ref_access_nested = nested.value,
            },
            chained1 = root_val,
            chained2 = chained1,
            non_existent_ref = does_not_exist,
            nested_non_existent_ref = nested.does_not_exist,
        "#;
        let gss = parse_str(source).expect("Should parse references successfully");

        // Test Expr::Symbol at root level
        assert_eq!(gss.get::<i32>(&["ref_symbol"]), Some(&42));

        // Test Expr::Symbol inside nested object
        assert_eq!(gss.get::<i32>(&["nested", "ref_symbol_nested"]), Some(&42));

        // Test Expr::Access at root level
        assert_eq!(gss.get::<i32>(&["ref_access"]), Some(&100));

        // Test Expr::Access inside nested object
        assert_eq!(gss.get::<i32>(&["other", "ref_access_nested"]), Some(&100));

        // Test chained references
        assert_eq!(gss.get::<i32>(&["chained2"]), Some(&42));

        // Test invalid reference (non-existent key)
        assert_eq!(gss.get::<i32>(&["non_existent_ref"]), None);
        assert_eq!(gss.get::<i32>(&["nested_non_existent_ref"]), None);

        // Test type mismatch
        assert_eq!(gss.get::<String>(&["ref_symbol"]), None);

        // Test dump with references
        gss.dump(0);
    }

    #[test]
    fn test_load_files() {
        let gss1 = load_gss_from_file("test/test.gss").expect("Should load test.gss");
        assert_eq!(gss1.get::<Percent>(&["style", "top"]), Some(&0.89));
        assert_eq!(gss1.get::<i32>(&["style", "count"]), Some(&69));
        assert_eq!(
            gss1.get::<String>(&["style", "inner", "link"]),
            Some(&"google.com".to_string())
        );

        let gss2 = load_gss_from_file("test/test2.gss").expect("Should load test2.gss");
        assert_eq!(gss2.get::<i32>(&["style", "image1", "top"]), Some(&50));
        assert_eq!(gss2.get::<i32>(&["style", "image2", "top"]), Some(&50));
        assert_eq!(gss2.get::<i32>(&["style", "image2", "left"]), Some(&50));
    }

    #[test]
    fn test_cycle_detection() {
        // Direct cycle: a = a,
        let source_direct = r#"
            a = a,
        "#;
        let gss = parse_str(source_direct).expect("Should parse");
        assert_eq!(gss.get::<i32>(&["a"]), None);

        // Indirect cycle: a = b, b = a,
        let source_indirect = r#"
            a = b,
            b = a,
        "#;
        let gss = parse_str(source_indirect).expect("Should parse");
        assert_eq!(gss.get::<i32>(&["a"]), None);
        assert_eq!(gss.get::<i32>(&["b"]), None);

        // Path cycle: a = b.x, b = { x = a },
        let source_path = r#"
            a = b.x,
            b = {
                x = a,
            },
        "#;
        let gss = parse_str(source_path).expect("Should parse");
        assert_eq!(gss.get::<i32>(&["a"]), None);
    }

    #[test]
    fn test_percent() {
        let source_path = r#"
            a = 89%,
        "#;
        let gss = parse_str(source_path).expect("Should parse");
        assert_eq!(gss.get::<Percent>(&["a"]), Some(&0.89));
    }

    #[test]
    fn test_float() {
        let source_path = r#"
            a = 0.123,
        "#;
        let gss = parse_str(source_path).expect("Should parse");
        assert_eq!(gss.get::<f32>(&["a"]), Some(&0.123));
    }

    #[test]
    fn test_get_fields() {
        let source_path = r#"
            a = 0.123,
            b = true,
            c = "test",
        "#;
        let gss = parse_str(source_path).expect("Should parse");
        for field in gss.get_fields() {
            assert!(["a", "b", "c"].contains(&field.as_str()))
        }
    }

    #[test]
    fn test_get_or_default() {
        let source_path = r#"

        "#;
        let gss = parse_str(source_path).expect("Should parse");
        assert_eq!(gss.get_or_default::<f32>(&["a"]), 0.0);
        assert_eq!(gss.get_or::<f32>(&["b"], 23.0), 23.0);
    }
}
