use std::any::Any;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fs;
use std::path::Path;

use lex_just_parse::lexer::*;
use lex_just_parse::parser::{Parser, RefLexer, many1, sep_by};
use lex_just_parse::try_parse;

#[cfg(not(feature = "value-enum"))]
pub type Value = Box<dyn std::any::Any + 'static>;

#[cfg(feature = "value-enum")]
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(u32),
    Float(f32),
    Bool(bool),
    String(String),
    Object(Object),
    Expr(Expr),
    Vec(Vec<Value>),
}

#[cfg(feature = "value-enum")]
impl Value {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        let any: &dyn std::any::Any = match self {
            Value::Number(x) => x,
            Value::Float(x) => x,
            Value::Bool(x) => x,
            Value::String(x) => x,
            Value::Object(x) => x,
            Value::Expr(x) => x,
            Value::Vec(x) => x,
        };
        any.downcast_ref::<T>()
    }
}

// Helper constructors to build Values under both configurations
fn new_number(x: u32) -> Value {
    #[cfg(not(feature = "value-enum"))]
    { Box::new(x) }
    #[cfg(feature = "value-enum")]
    { Value::Number(x) }
}

fn new_float(x: f32) -> Value {
    #[cfg(not(feature = "value-enum"))]
    { Box::new(x) }
    #[cfg(feature = "value-enum")]
    { Value::Float(x) }
}

fn new_bool(x: bool) -> Value {
    #[cfg(not(feature = "value-enum"))]
    { Box::new(x) }
    #[cfg(feature = "value-enum")]
    { Value::Bool(x) }
}

fn new_string(x: String) -> Value {
    #[cfg(not(feature = "value-enum"))]
    { Box::new(x) }
    #[cfg(feature = "value-enum")]
    { Value::String(x) }
}

fn new_object(x: Object) -> Value {
    #[cfg(not(feature = "value-enum"))]
    { Box::new(x) }
    #[cfg(feature = "value-enum")]
    { Value::Object(x) }
}

fn new_expr(x: Expr) -> Value {
    #[cfg(not(feature = "value-enum"))]
    { Box::new(x) }
    #[cfg(feature = "value-enum")]
    { Value::Expr(x) }
}

fn new_vec(x: Vec<Value>) -> Value {
    #[cfg(not(feature = "value-enum"))]
    { Box::new(x) }
    #[cfg(feature = "value-enum")]
    { Value::Vec(x) }
}
pub type Percent = f32;
pub type Gss = Object;

pub trait FromGssValue: Sized {
    fn from_gss_value(value: &Value) -> Option<Self>;
}

macro_rules! impl_from_gss_for_int {
    ($($t:ty),*) => {
        $(
            impl FromGssValue for $t {
                fn from_gss_value(value: &Value) -> Option<Self> {
                    if let Some(x) = value.downcast_ref::<u32>() {
                        <$t>::try_from(*x).ok()
                    } else if let Some(x) = value.downcast_ref::<f32>() {
                        if x.fract() == 0.0 && *x >= (<$t>::MIN as f32) && *x <= (<$t>::MAX as f32) {
                            Some(*x as $t)
                        } else {
                            None
                        }
                    } else if let Some(x) = value.downcast_ref::<$t>() {
                        Some(*x)
                    } else {
                        None
                    }
                }
            }
        )*
    };
}

impl_from_gss_for_int!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

impl FromGssValue for f32 {
    fn from_gss_value(value: &Value) -> Option<Self> {
        if let Some(x) = value.downcast_ref::<f32>() {
            Some(*x)
        } else if let Some(x) = value.downcast_ref::<u32>() {
            Some(*x as f32)
        } else {
            None
        }
    }
}

impl FromGssValue for f64 {
    fn from_gss_value(value: &Value) -> Option<Self> {
        if let Some(x) = value.downcast_ref::<f32>() {
            Some(*x as f64)
        } else if let Some(x) = value.downcast_ref::<u32>() {
            Some(*x as f64)
        } else if let Some(x) = value.downcast_ref::<f64>() {
            Some(*x)
        } else {
            None
        }
    }
}

impl FromGssValue for bool {
    fn from_gss_value(value: &Value) -> Option<Self> {
        value.downcast_ref::<bool>().copied()
    }
}

impl FromGssValue for String {
    fn from_gss_value(value: &Value) -> Option<Self> {
        value.downcast_ref::<String>().cloned()
    }
}
impl<T: FromGssValue> FromGssValue for Vec<T> {
    fn from_gss_value(value: &Value) -> Option<Self> {
        value.downcast_ref::<Vec<Value>>()?
            .iter()
            .map(T::from_gss_value)
            .collect()
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "value-enum", derive(Clone, PartialEq))]
pub struct Object {
    inner: HashMap<String, Value>,
    max_depth: usize,
    allow_redefinition: bool,
}

#[derive(Debug)]
#[cfg_attr(feature = "value-enum", derive(Clone, PartialEq))]
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
            allow_redefinition: false,
        }
    }

    pub fn with_allow_redefinition(mut self, allow: bool) -> Self {
        self.allow_redefinition = allow;
        self
    }

    pub fn set_allow_redefinition(&mut self, allow: bool) {
        self.allow_redefinition = allow;
    }

    pub fn allow_redefinition(&self) -> bool {
        self.allow_redefinition
    }
}

fn print_value_for_dump(v: &Value, level: usize) {
    if let Some(string) = v.downcast_ref::<String>() {
        print!("\"{}\"", string);
    } else if let Some(i) = v.downcast_ref::<u32>() {
        print!("{}", i);
    } else if let Some(i) = v.downcast_ref::<f32>() {
        print!("{}", i);
    } else if let Some(b) = v.downcast_ref::<bool>() {
        print!("{}", b);
    } else if let Some(obj) = v.downcast_ref::<Object>() {
        obj.dump(level + 1);
    } else if let Some(expr) = v.downcast_ref::<Expr>() {
        match expr {
            Expr::Symbol(s) => print!("{s}"),
            Expr::Access(seq) => {
                for (i, s) in seq.iter().enumerate() {
                    if i > 0 {
                        print!(".");
                    }
                    print!("{s}");
                }
            }
            Expr::RelAccess(seq) => {
                print!(".");
                for (i, s) in seq.iter().enumerate() {
                    if i > 0 {
                        print!(".");
                    }
                    print!("{s}");
                }
            }
        }
    } else if let Some(vec) = v.downcast_ref::<Vec<Value>>() {
        print!("[");
        for (i, val) in vec.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print_value_for_dump(val, level);
        }
        print!("]");
    } else {
        print!("UNKNOWN({:?})", std::any::Any::type_id(v));
    }
}

impl Object {
    pub fn dump(&self, level: usize) {
        println!("{{");

        for (k, v) in self.inner.iter() {
            for _ in 0..=level {
                print!("    ");
            }

            print!("{k} => ");
            print_value_for_dump(v, level);
            println!();
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
        self.get_value_impl(path, 0, self.max_depth)
            .and_then(|v| v.downcast_ref::<T>())
    }

    pub fn get_as<T: FromGssValue>(&self, path: &[&str]) -> Option<T> {
        self.get_value_impl(path, 0, self.max_depth)
            .and_then(T::from_gss_value)
    }

    pub fn get_or_default<T: FromGssValue + Default>(&self, path: &[&str]) -> T {
        self.get_as::<T>(path).unwrap_or_default()
    }

    pub fn get_or<T: FromGssValue>(&self, path: &[&str], default: T) -> T {
        self.get_as::<T>(path).unwrap_or(default)
    }

    pub fn get_value(&self, path: &[&str]) -> Option<&Value> {
        self.get_value_impl(path, 0, self.max_depth)
    }

    fn get_value_impl<'a>(
        &'a self,
        path: &[&str],
        current_depth: usize,
        max_depth: usize,
    ) -> Option<&'a Value> {
        if current_depth >= max_depth {
            return None;
        }
        let mut obj = self;
        if let Some((last, prefix)) = path.split_last() {
            for c in prefix {
                if let Some(v) = obj.inner.get(*c) {
                    if let Some(o) = v.downcast_ref::<Object>() {
                        obj = o;
                    } else if let Some(expr) = v.downcast_ref::<Expr>() {
                        obj = match expr {
                            Expr::Symbol(s) => {
                                let val = self.get_value_impl(&[s.as_str()], current_depth + 1, max_depth)?;
                                val.downcast_ref::<Object>()?
                            }
                            Expr::Access(seq) => {
                                let tmp: Vec<&str> = seq.iter().map(AsRef::as_ref).collect();
                                let val = self.get_value_impl(&tmp, current_depth + 1, max_depth)?;
                                val.downcast_ref::<Object>()?
                            }
                            Expr::RelAccess(seq) => {
                                let tmp: Vec<&str> = seq.iter().map(AsRef::as_ref).collect();
                                let val = obj.get_value_impl(&tmp, current_depth + 1, max_depth)?;
                                val.downcast_ref::<Object>()?
                            }
                        };
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
                            self.get_value_impl(&[s.as_str()], current_depth + 1, max_depth)
                        }
                        Expr::Access(seq) => {
                            let tmp: Vec<&str> = seq.iter().map(AsRef::as_ref).collect();
                            self.get_value_impl(&tmp, current_depth + 1, max_depth)
                        }
                        Expr::RelAccess(seq) => {
                            let tmp: Vec<&str> = seq.iter().map(AsRef::as_ref).collect();
                            obj.get_value_impl(&tmp, current_depth + 1, max_depth)
                        }
                    };
                }
                return Some(v);
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
    load_gss_from_file_with_options(file_path, false)
}

pub fn load_gss_from_file_with_options<P: AsRef<Path>>(
    file_path: P,
    allow_redefinition: bool,
) -> Result<Gss, Box<dyn StdError>> {
    let source = fs::read_to_string(file_path.as_ref())?;
    let mut lex = get_lexer(
        &source,
        #[cfg(feature = "interning")]
        &file_path,
    );

    let gss = parse(file_path, &mut lex, allow_redefinition)?;

    Ok(gss)
}

/// Parses a GSS string into a `Gss` style context.
pub fn parse_str(source: &str) -> Result<Gss, Box<dyn StdError>> {
    parse_str_with_options(source, false)
}

/// Parses a GSS string with custom parser options into a `Gss` style context.
pub fn parse_str_with_options(source: &str, allow_redefinition: bool) -> Result<Gss, Box<dyn StdError>> {
    let mut lex = get_lexer(
        source,
        #[cfg(feature = "interning")]
        "<input>",
    );

    parse("<input>", &mut lex, allow_redefinition)
}

fn get_lexer<#[cfg(feature = "interning")] P: AsRef<Path>>(
    source: &str,
    #[cfg(feature = "interning")] file_path: P,
) -> Lexer<'_> {
    #[cfg(not(feature = "interning"))]
    let lex = Lexer::new(source);
    #[cfg(feature = "interning")]
    let lex = Lexer::new(file_path.as_ref().to_string_lossy(), source);
    lex
}

#[cfg(feature = "internal-api")]
pub fn internal_parse<'lex, P: AsRef<Path>>(
    file_path: P,
    lex: RefLexer<'lex>,
) -> Result<Gss, Box<dyn StdError>> {
    parse(file_path, lex, false)
}

fn parse<'lex, P: AsRef<Path>>(
    file_path: P,
    lex: RefLexer<'lex>,
    allow_redefinition: bool,
) -> Result<Gss, Box<dyn StdError>> {
    match parse_gss(lex, allow_redefinition) {
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

#[cfg(feature = "internal-api")]
pub fn internal_parse_gss<'lex>(lex: RefLexer) -> Parser<Gss, Box<dyn StdError>> {
    parse_gss(lex, false)
}

fn parse_gss<'lex>(mut lex: RefLexer, allow_redefinition: bool) -> Parser<Gss, Box<dyn StdError>> {
    let mut object = Object::new();
    object.set_allow_redefinition(allow_redefinition);
    if lex.peek().kind == TokenKind::EOF {
        return Parser::Success(lex, object);
    }
    let fields = try_parse!(
        lex,
        many1(lex, |mut lex| {
            let k = try_parse!(lex, parse_ident(lex));
            try_parse!(lex, parse_eq(lex));
            let v = try_parse!(lex, parse_value(lex, allow_redefinition));
            if lex.peek().kind == TokenKind::Comma {
                lex.next();
            }
            Parser::Success(lex, (k, v))
        })
    );
    try_parse!(lex, parse_eof(lex));
    for (key, value) in fields {
        if object.inner.insert(key.to_string(), value).is_some() && !allow_redefinition {
            return Parser::Fail(lex, format!("Redefinition of key {key}").into());
        }
    }
    Parser::Success(lex, object)
}

#[cfg(feature = "internal-api")]
pub fn internal_parse_object<'lex>(lex: RefLexer) -> Parser<Object, Box<dyn StdError>> {
    parse_object(lex, false)
}

#[cfg(feature = "internal-api")]
pub fn internal_parse_object_from_str<'lex>(s: &str) -> Result<Object, Box<dyn StdError>> {
    let mut lex = get_lexer(
        s,
        #[cfg(feature = "interning")]
        "<object_from_str>",
    );
    parse_object(&mut lex, false).success().map_err(|(_, err)| err)
}

fn parse_object<'lex>(mut lex: RefLexer, allow_redefinition: bool) -> Parser<Object, Box<dyn StdError>> {
    let mut object = Object::new();
    object.set_allow_redefinition(allow_redefinition);
    if lex.peek().kind == TokenKind::OpenCurly {
        try_parse!(lex, parse_open_curly(lex));
    }
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
                let v = try_parse!(lex, parse_value(lex, allow_redefinition));
                Parser::Success(lex, (k, v))
            },
            parse_maybe_comma
        )
    );
    try_parse!(lex, parse_close_curly(lex));
    for (key, value) in fields {
        if object.inner.insert(key.to_string(), value).is_some() && !allow_redefinition {
            return Parser::Fail(lex, format!("Redefinition of key {key}").into());
        }
    }
    Parser::Success(lex, object)
}

fn parse_list<'lex>(mut lex: RefLexer, allow_redefinition: bool) -> Parser<Vec<Value>, Box<dyn StdError>> {
    try_parse!(lex, parse_open_bracket(lex));
    if lex.peek().kind == TokenKind::CloseBracket {
        lex.next();
        return Parser::Success(lex, Vec::new());
    }
    let items = try_parse!(
        lex,
        sep_by(
            lex,
            |mut lex| {
                if lex.peek().kind == TokenKind::CloseBracket {
                    return Parser::Fail(lex, "End of list".into());
                }
                let v = try_parse!(lex, parse_value(lex, allow_redefinition));
                Parser::Success(lex, v)
            },
            parse_maybe_comma
        )
    );
    try_parse!(lex, parse_close_bracket(lex));
    Parser::Success(lex, items)
}


#[cfg(feature = "internal-api")]
pub fn internal_parse_value<'lex>(lex: RefLexer) -> Parser<Value, Box<dyn StdError>> {
    parse_value(lex, false)
}

fn parse_value<'lex>(mut lex: RefLexer, allow_redefinition: bool) -> Parser<Value, Box<dyn StdError>> {
    if lex.peek().kind == TokenKind::OpenBracket {
        let list = try_parse!(lex, parse_list(lex, allow_redefinition));
        return Parser::Success(lex, new_vec(list));
    }
    let t = lex.next();
    match t.kind {
        TokenKind::Number(base) => {
            let x = match u32::from_str_radix(t.source(), base.radix()) {
                Ok(x) => x,
                Err(err) => return Parser::Fail(lex, err.into()),
            };
            if lex.peek().kind == TokenKind::Mod {
                lex.next();
                return Parser::Success(lex, new_float(x as f32 / 100.));
            }
            Parser::Success(lex, new_number(x))
        }
        TokenKind::RealNumber => {
            let x = match t.source().parse::<f32>() {
                Ok(x) => x,
                Err(err) => return Parser::Fail(lex, err.into()),
            };
            if lex.peek().kind == TokenKind::Mod {
                lex.next();
                return Parser::Success(lex, new_float(x as f32 / 100.));
            }
            Parser::Success(lex, new_float(x))
        }
        TokenKind::Identifier if t.source() == "true" => Parser::Success(lex, new_bool(true)),
        TokenKind::Identifier if t.source() == "false" => Parser::Success(lex, new_bool(false)),
        TokenKind::StringLiteral => Parser::Success(lex, new_string(t.unescape())),
        TokenKind::OpenCurly => {
            let object = try_parse!(lex, parse_object(lex, allow_redefinition));
            Parser::Success(lex, new_object(object))
        }
        TokenKind::OpenBracket => {
            let list = try_parse!(lex, parse_list(lex, allow_redefinition));
            Parser::Success(lex, new_vec(list))
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
                return Parser::Success(lex, new_expr(Expr::Access(seq)));
            }
            Parser::Success(lex, new_expr(Expr::Symbol(t.source.to_string())))
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
                return Parser::Success(lex, new_expr(Expr::RelAccess(seq)));
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

make_expect! {parse_open_bracket, TokenKind::OpenBracket, "[" }
make_expect! {parse_close_bracket, TokenKind::CloseBracket, "]" }
make_expect! {parse_dot, TokenKind::Dot, "." }
make_expect! {parse_comma, TokenKind::Comma, "," }
make_expect! {parse_eq, TokenKind::Eq, "=" }
make_expect! {parse_open_curly, TokenKind::OpenCurly, "{" }
make_expect! {parse_close_curly, TokenKind::CloseCurly, "}" }
make_expect! {parse_eof, TokenKind::EOF, "EOF" }
make_expect! {ret, parse_ident, TokenKind::Identifier, "identifier" }

fn parse_maybe_comma<'lex>(lex: RefLexer) -> Parser<(), Box<dyn StdError>> {
    if lex.peek().kind == TokenKind::Comma {
        return parse_comma(lex);
    }
    Parser::Success(lex, ())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(gss.get::<u32>(&["version"]), Some(&1));
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
        assert_eq!(gss.get::<u32>(&["active"]), None); // Type mismatch
    }

    #[test]
    fn test_parse_list() {
        let source = r#"
            numbers = [1, 2, 3],
            mixed = ["hello", 42, true],
            empty = [],
            nested = [[1, 2], [3, 4]],
        "#;
        let gss = parse_str(source).expect("Should parse lists successfully");

        assert_eq!(gss.get_as::<Vec<u32>>(&["numbers"]), Some(vec![1, 2, 3]));
        assert!(gss.get::<Vec<Value>>(&["mixed"]).is_some());
        assert_eq!(gss.get_as::<Vec<u32>>(&["empty"]), Some(vec![]));
        assert_eq!(
            gss.get_as::<Vec<Vec<u32>>>(&["nested"]),
            Some(vec![vec![1, 2], vec![3, 4]])
        );
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
        assert!(result.is_ok());
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
        assert_eq!(gss.get::<u32>(&[]), None);
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
        assert_eq!(gss.get::<u32>(&["ref_symbol"]), Some(&42));

        // Test Expr::Symbol inside nested object
        assert_eq!(gss.get::<u32>(&["nested", "ref_symbol_nested"]), Some(&42));

        // Test Expr::Access at root level
        assert_eq!(gss.get::<u32>(&["ref_access"]), Some(&100));

        // Test Expr::Access inside nested object
        assert_eq!(gss.get::<u32>(&["other", "ref_access_nested"]), Some(&100));

        // Test chained references
        assert_eq!(gss.get::<u32>(&["chained2"]), Some(&42));

        // Test invalid reference (non-existent key)
        assert_eq!(gss.get::<u32>(&["non_existent_ref"]), None);
        assert_eq!(gss.get::<u32>(&["nested_non_existent_ref"]), None);

        // Test type mismatch
        assert_eq!(gss.get::<String>(&["ref_symbol"]), None);

        // Test dump with references
        gss.dump(0);
    }

    #[test]
    fn test_load_files() {
        let gss1 = load_gss_from_file("test/test.gss").expect("Should load test.gss");
        assert_eq!(gss1.get::<Percent>(&["style", "top"]), Some(&0.89));
        assert_eq!(gss1.get::<u32>(&["style", "count"]), Some(&69));
        assert_eq!(
            gss1.get::<String>(&["style", "inner", "link"]),
            Some(&"google.com".to_string())
        );

        let gss2 = load_gss_from_file("test/test2.gss").expect("Should load test2.gss");
        assert_eq!(gss2.get::<u32>(&["style", "image1", "top"]), Some(&50));
        assert_eq!(gss2.get::<u32>(&["style", "image2", "top"]), Some(&50));
        assert_eq!(gss2.get::<u32>(&["style", "image2", "left"]), Some(&50));

        let gss3 = load_gss_from_file("test/test3.gss").expect("Should load test3.gss");
        assert_eq!(gss3.get::<u32>(&["test", "key"]), Some(&1));
        assert_eq!(gss3.get::<u32>(&["test", "other"]), Some(&2));
        assert_eq!(gss3.get::<u32>(&["test", "hex"]), Some(&0x32));
    }

    #[test]
    fn test_cycle_detection() {
        // Direct cycle: a = a,
        let source_direct = r#"
            a = a,
        "#;
        let gss = parse_str(source_direct).expect("Should parse");
        assert_eq!(gss.get::<u32>(&["a"]), None);

        // Indirect cycle: a = b, b = a,
        let source_indirect = r#"
            a = b,
            b = a,
        "#;
        let gss = parse_str(source_indirect).expect("Should parse");
        assert_eq!(gss.get::<u32>(&["a"]), None);
        assert_eq!(gss.get::<u32>(&["b"]), None);

        // Path cycle: a = b.x, b = { x = a },
        let source_path = r#"
            a = b.x,
            b = {
                x = a,
            },
        "#;
        let gss = parse_str(source_path).expect("Should parse");
        assert_eq!(gss.get::<u32>(&["a"]), None);
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

    #[test]
    fn test_get_inner_obj() {
        let source_path = r#"
            h = {
                inner = "Hi"
            }
            f = {
                g = h
            }
        "#;
        let gss = parse_str(source_path).expect("Should parse");
        assert_eq!(
            gss.get_or_default::<String>(&["f", "g", "inner"]),
            "Hi".to_string()
        );
    }

    #[test]
    fn test_dot_separated_lookup() {
        let source = r#"
            a = {
                b = {
                    c = "found_it"
                }
                d = {
                    e = a.b.c
                }
            }
        "#;
        let gss = parse_str(source).expect("Should parse");
        assert_eq!(
            gss.get::<String>(&["a", "b", "c"]),
            Some(&"found_it".to_string())
        );
        assert_eq!(
            gss.get::<String>(&["a", "b", "c"]),
            Some(&"found_it".to_string())
        );
        assert_eq!(
            gss.get::<String>(&["a", "d", "e"]),
            Some(&"found_it".to_string())
        );
    }

    #[test]
    fn test_get_as_common_types() {
        let source = r#"
            num = 42,
            float_val = 3.14,
            text = "antigravity",
            flag = true,
            big_num = 300,
        "#;
        let gss = parse_str(source).expect("Should parse");

        // Signed integer conversions
        assert_eq!(gss.get_as::<i8>(&["num"]), Some(42i8));
        assert_eq!(gss.get_as::<i16>(&["num"]), Some(42i16));
        assert_eq!(gss.get_as::<i32>(&["num"]), Some(42i32));
        assert_eq!(gss.get_as::<i64>(&["num"]), Some(42i64));
        assert_eq!(gss.get_as::<i128>(&["num"]), Some(42i128));
        assert_eq!(gss.get_as::<isize>(&["num"]), Some(42isize));

        // Unsigned integer conversions
        assert_eq!(gss.get_as::<u8>(&["num"]), Some(42u8));
        assert_eq!(gss.get_as::<u16>(&["num"]), Some(42u16));
        assert_eq!(gss.get_as::<u32>(&["num"]), Some(42u32));
        assert_eq!(gss.get_as::<u64>(&["num"]), Some(42u64));
        assert_eq!(gss.get_as::<u128>(&["num"]), Some(42u128));
        assert_eq!(gss.get_as::<usize>(&["num"]), Some(42usize));

        // Float conversions
        assert_eq!(gss.get_as::<f32>(&["float_val"]), Some(3.14f32));
        assert_eq!(gss.get_as::<f64>(&["float_val"]), Some(3.14f32 as f64));
        assert_eq!(gss.get_as::<f32>(&["num"]), Some(42.0f32));
        assert_eq!(gss.get_as::<f64>(&["num"]), Some(42.0f64));

        // Booleans & Strings
        assert_eq!(gss.get_as::<bool>(&["flag"]), Some(true));
        assert_eq!(gss.get_as::<String>(&["text"]), Some("antigravity".to_string()));

        // Bounds checking
        assert_eq!(gss.get_as::<u8>(&["big_num"]), None); // 300 > u8::MAX
        assert_eq!(gss.get_as::<i8>(&["big_num"]), None); // 300 > i8::MAX
        assert_eq!(gss.get_as::<u16>(&["big_num"]), Some(300u16));

        // Fallbacks with get_or and get_or_default
        assert_eq!(gss.get_or::<i32>(&["num"], 0), 42);
        assert_eq!(gss.get_or::<i32>(&["missing"], 99), 99);
        assert_eq!(gss.get_or_default::<i64>(&["num"]), 42i64);
        assert_eq!(gss.get_or_default::<i64>(&["missing"]), 0i64);
    }

    #[test]
    fn test_allow_redefinition() {
        let source_root = r#"
            key = 1,
            key = 2,
        "#;
        // Default (false) should fail
        assert!(parse_str(source_root).is_err());

        // With allow_redefinition = true
        let gss = parse_str_with_options(source_root, true).expect("Should allow redefinition");
        assert_eq!(gss.get_as::<u32>(&["key"]), Some(2));
        assert!(gss.allow_redefinition());

        // Nested redefinition
        let source_nested = r#"
            obj = {
                a = 10,
                a = 20,
            }
        "#;
        assert!(parse_str(source_nested).is_err());
        let gss_nested = parse_str_with_options(source_nested, true).expect("Should allow nested redefinition");
        assert_eq!(gss_nested.get_as::<u32>(&["obj", "a"]), Some(20));

        // Builder & Setter
        let mut obj = Object::new().with_allow_redefinition(true);
        assert!(obj.allow_redefinition());
        obj.set_allow_redefinition(false);
        assert!(!obj.allow_redefinition());
    }

    #[cfg(feature = "value-enum")]
    #[test]
    fn test_value_enum_specifics() {
        let source = "a = 42, b = { x = 1 }, c = \"hello\"";
        let gss = parse_str(source).unwrap();
        
        let a_val = gss.get_value(&["a"]).unwrap();
        let b_val = gss.get_value(&["b"]).unwrap();
        let c_val = gss.get_value(&["c"]).unwrap();
        
        // Assert equality works directly on Value
        assert_eq!(a_val, &Value::Number(42));
        assert_eq!(c_val, &Value::String("hello".to_string()));
        
        // Assert cloning works directly on Value
        let a_clone = a_val.clone();
        assert_eq!(a_clone, Value::Number(42));
        
        let b_clone = b_val.clone();
        if let Value::Object(ref obj) = b_clone {
            assert_eq!(obj.get::<u32>(&["x"]), Some(&1));
        } else {
            panic!("Expected Value::Object");
        }
    }
}
