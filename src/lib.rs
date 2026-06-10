use std::any::Any;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fs;
use std::path::Path;

use lex_just_parse::lexer::{Lexer, TokenKind};
use lex_just_parse::parser::{Parser, RefLexer};
use lex_just_parse::try_parse;

pub type Value = Box<dyn Any + 'static>;
pub type Gss = Object;

#[derive(Debug)]
pub struct Object(pub HashMap<String, Value>);

#[derive(Debug)]
pub enum Expr {
    Symbol(String),
    Access(Vec<String>),
}

impl Object {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn dump(&self, level: usize) {
        println!("{{");

        for (k, v) in self.0.iter() {
            for _ in 0..=level {
                print!("    ");
            }

            print!("{k} => ");

            if let Some(string) = v.downcast_ref::<String>() {
                println!("{}", string);
            } else if let Some(i) = v.downcast_ref::<i32>() {
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
                }
            } else {
                println!("UNKNOW({:?})", v.type_id());
            }
        }

        for _ in 0..level {
            print!("    ");
        }

        println!("}}");
    }

    pub fn get<T: 'static>(&self, path: &[&str]) -> Option<&T> {
        let mut obj = self;
        for c in path.iter().rev().skip(1).rev() {
            let v = obj.0.get(*c)?;
            if let Some(o) = v.downcast_ref::<Object>() {
                obj = o;
            } else {
                return None;
            }
        }
        if let Some(last) = path.last() {
            if let Some(v) = obj.0.get(*last) {
                if let Some(expr) = v.downcast_ref::<Expr>() {
                    match expr {
                        Expr::Symbol(s) => return self.get(&[s.as_str()]),
                        Expr::Access(seq) => {
                            let tmp: Vec<&str> = seq.iter().map(AsRef::as_ref).collect();
                            return self.get(&tmp);
                        }
                    }
                }
                return v.downcast_ref::<T>();
            }
        }
        None
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
    match parse_object(lex) {
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

fn parse_object<'lex>(mut lex: RefLexer) -> Parser<Gss, Box<dyn StdError>> {
    let mut object = Object::new();
    loop {
        let t = lex.peek();
        if t.kind == TokenKind::EOF {
            lex.next();
            break;
        }
        let (l, ()) = try_parse!(expect(lex, TokenKind::Identifier));
        let key = l.next().source;
        let (l, _) = try_parse!(expect(l, TokenKind::Eq));
        l.next();
        let (l, value) = try_parse!(parse_value(l));
        let (l, _) = try_parse!(expect(l, TokenKind::Comma));
        l.next();
        if object.0.insert(key.clone(), value).is_some() {
            return Parser::Fail(l, format!("Redefinition of key {key}").into());
        }
        lex = l;
        let t = lex.peek();
        if t.kind == TokenKind::CloseCurly {
            break;
        }
    }
    Parser::Success(lex, object)
}

fn parse_value<'lex>(mut lex: RefLexer) -> Parser<Value, Box<dyn StdError>> {
    let t = lex.next();
    match t.kind {
        TokenKind::Int(base) => {
            let x = match i32::from_str_radix(t.source(), base.radix()) {
                Ok(x) => x,
                Err(err) => return Parser::Fail(lex, err.into()),
            };
            Parser::Success(lex, Box::new(x))
        }
        TokenKind::Identifier if t.source() == "true" => Parser::Success(lex, Box::new(true)),
        TokenKind::Identifier if t.source() == "false" => Parser::Success(lex, Box::new(false)),
        TokenKind::StringLiteral => Parser::Success(lex, Box::new(t.unescape())),
        TokenKind::OpenCurly => {
            let (lex, object) = try_parse!(parse_object(lex));
            let (lex, _) = try_parse!(expect(lex, TokenKind::CloseCurly));
            lex.next();
            Parser::Success(lex, Box::new(object))
        }
        TokenKind::Identifier => {
            if lex.peek().kind == TokenKind::Dot {
                let mut seq = vec![t.source];
                while lex.peek().kind == TokenKind::Dot {
                    lex.next();
                    let (l, _) = try_parse!(expect(lex, TokenKind::Identifier));
                    let t = l.next();
                    lex = l;
                    seq.push(t.source);
                }
                return Parser::Success(lex, Box::new(Expr::Access(seq)));
            }
            Parser::Success(lex, Box::new(Expr::Symbol(t.source)))
        }
        _ => {
            return Parser::Fail(lex, format!("Unexpect token `{t}`").into());
        }
    }
}

fn expect<'lex>(lex: RefLexer, expect: TokenKind) -> Parser<(), Box<dyn StdError>> {
    let actual = lex.peek().kind;
    if actual != expect {
        return Parser::Fail(lex, format!("Expect {expect:?} got {actual:?}").into());
    }
    Parser::Success(lex, ())
}

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
            key = 1
            other = 2,
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
        assert_eq!(gss1.get::<i32>(&["style", "top"]), Some(&69));
        assert_eq!(gss1.get::<String>(&["style", "inner", "link"]), Some(&"google.com".to_string()));

        let gss2 = load_gss_from_file("test/test2.gss").expect("Should load test2.gss");
        assert_eq!(gss2.get::<i32>(&["style", "image1", "top"]), Some(&50));
        assert_eq!(gss2.get::<i32>(&["style", "image2", "top"]), Some(&50));
        assert_eq!(gss2.get::<i32>(&["style", "image2", "left"]), Some(&50));
    }
}
