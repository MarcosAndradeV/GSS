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

fn parse_value<'lex>(lex: RefLexer) -> Parser<Value, Box<dyn StdError>> {
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
        _ => todo!(),
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
    // use super::*;

    #[test]
    fn test_parse() {}
}
