# Graph Style Sheets (GSS)

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A lightweight, expressive stylesheet and declarative object configuration language parser written in Rust.

Inspired by an idea from [Tsoding](https://twitch.tv/tsoding).

---

## Table of Contents

- [Overview](#overview)
- [Key Features](#key-features)
- [GSS Language Syntax](#gss-language-syntax)
  - [Value Types](#value-types)
  - [Nested Objects](#nested-objects)
  - [References and Expressions](#references-and-expressions)
- [Rust API Reference](#rust-api-reference)
  - [Loading & Parsing](#loading--parsing)
  - [Querying Values](#querying-values)
  - [Flexible Type Conversion with `FromGssValue`](#flexible-type-conversion-with-fromgssvalue)
  - [Allowing Key Redefinitions](#allowing-key-redefinitions)
  - [Cycle Detection & Depth Limiting](#cycle-detection--depth-limiting)
  - [Inspection & Debugging](#inspection--debugging)
- [Type Mapping & Conversion Reference](#type-mapping--conversion-reference)
- [Cargo Features](#cargo-features)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Adding as a Dependency](#adding-as-a-dependency)
  - [Code Example](#code-example)
- [CLI / Example Binary](#cli--example-binary)
- [Architecture](#architecture)
- [Project Layout](#project-layout)
- [Development & Testing](#development--testing)
- [Troubleshooting & Common Pitfalls](#troubleshooting--common-pitfalls)
- [License](#license)

---

## Overview

**Graph Style Sheets (GSS)** provides a simple yet powerful declarative format for configuring hierarchical and graph-like structures. Unlike static serialization formats like JSON or TOML, GSS natively supports **dynamic symbol resolution**, **absolute and relative property references**, **percentages**, **cycle protection**, and **flexible Rust type conversion**.

---

## Key Features

- **Rich Literal Types**: Integers (including hexadecimal radix `0x...`), floating-point numbers, percentages (`89%` $\rightarrow$ `0.89`), booleans (`true`/`false`), and unescaped strings.
- **Nested Hierarchies**: Arbitrarily deep nested objects with flexible comma handling.
- **First-Class Symbol Resolution**:
  - **Root References**: `ref = root_value`
  - **Absolute Dot Access**: `target = style.button.background`
  - **Relative / Sibling Dot Access**: `height = .width`
  - **Chained Lookups**: Transparently traverses reference chains across objects.
- **Cycle Detection**: Configurable recursion depth limit (`max_depth`, default `20`) prevents infinite loops caused by circular dependencies (`a = b, b = a`).
- **Flexible Key Redefinition Option**: Strict parse-time validation by default, with an option (`allow_redefinition`) to allow overriding duplicate keys.
- **Ergonomic Rust API**:
  - Exact reference downcasting via `.get::<T>()`.
  - Type-converting getters (`.get_as::<T>()`, `.get_or::<T>()`, `.get_or_default::<T>()`) supporting all common Rust integer types (`i8`-`i128`, `isize`, `u8`-`u128`, `usize`), floats (`f32`, `f64`), `bool`, and `String`.

---

## GSS Language Syntax

GSS files consist of top-level key-value assignments:

```gss
key = value,
```

Trailing and separating commas are optional in object definitions.

### Value Types

| Type | Syntax Example | Rust Storage Type | Supported `get_as::<T>()` Conversions |
| :--- | :--- | :--- | :--- |
| **Integer** | `count = 42,`<br>`hex = 0x32,` | `u32` | `i8`..`i128`, `isize`, `u8`..`u128`, `usize`, `f32`, `f64` |
| **Float** | `price = 42.12,`<br>`scale = 0.5,` | `f32` | `f32`, `f64`, integer types (if whole number) |
| **Percentage** | `width = 89%,` | `Percent` (`f32`) | `Percent` (`f32`), `f64` (evaluates as `0.89`) |
| **Boolean** | `visible = true,`<br>`debug = false,` | `bool` | `bool` |
| **String** | `title = "Hello World",` | `String` | `String` |
| **Object** | `nested = { ... }` | `Object` / `Gss` | `&Object` (via `get::<Object>()`) |

### Nested Objects

Objects can be nested to any depth:

```gss
style = {
    top = 89%,
    count = 69,
    price = 42.12,
    frame = true,
    inner = {
        link = "google.com",
    },
},
```

### References and Expressions

GSS allows values to dynamically reference other values:

#### 1. Root / Symbol Reference
Refers to a key defined at the root level of the GSS context:
```gss
base_color = "red",
button = {
    color = base_color,
},
```

#### 2. Absolute Dot Path Access
Navigates from the root object through nested fields:
```gss
style = {
    image1 = {
        top = 50,
        left = 50,
    },
    image2 = {
        top = style.image1.top,
        left = style.image1.left,
    },
},
```

#### 3. Relative Sibling Dot Access
Prefixing with a dot (`.`) performs a lookup relative to the current object scope:
```gss
style = {
    image3 = {
        left = 50,
        top = .left,  // Resolves to 50 from the same object
    },
},
```

#### 4. Chained References
References can chain across multiple keys:
```gss
root_val = 42,
chained1 = root_val,
chained2 = chained1,  // Resolves to 42
```

---

## Rust API Reference

### Loading & Parsing

```rust
use gss::{load_gss_from_file, parse_str, parse_str_with_options, Gss};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Parse directly from a string (strict mode: no duplicate keys)
    let gss: Gss = parse_str(r#"
        window = {
            width = 800,
            height = 600,
        }
    "#)?;

    // 2. Parse from a string allowing key redefinitions
    let gss_redef: Gss = parse_str_with_options("key = 1, key = 2,", true)?;

    // 3. Or load from a file
    let gss_from_file: Gss = load_gss_from_file("config.gss")?;

    Ok(())
}
```

### Querying Values

Query values by providing a path of string slices (`&[&str]`):

```rust
// 1. Converted value getter: get_as::<T>()
// Automatically converts between integer and float types!
if let Some(width) = gss.get_as::<i32>(&["window", "width"]) {
    println!("Width (i32): {width}");
}

// 2. Exact reference getter: get::<T>()
// Returns Option<&T> matching the exact underlying stored type
if let Some(width_ref) = gss.get::<u32>(&["window", "width"]) {
    println!("Width ref (&u32): {width_ref}");
}

// 3. Fallback getter: get_or::<T>()
let height: i64 = gss.get_or::<i64>(&["window", "height"], 480);

// 4. Default fallback: get_or_default::<T>()
let title: String = gss.get_or_default::<String>(&["window", "title"]);
```

### Flexible Type Conversion with `FromGssValue`

The `FromGssValue` trait allows `.get_as::<T>()`, `.get_or::<T>()`, and `.get_or_default::<T>()` to convert values seamlessly into any target Rust type:

```rust
let gss = parse_str("count = 42, price = 19.95,")?;

// Retrieve as any integer type:
let as_i32: Option<i32> = gss.get_as(&["count"]);     // Some(42)
let as_usize: Option<usize> = gss.get_as(&["count"]); // Some(42)
let as_i64: Option<i64> = gss.get_as(&["count"]);     // Some(42)

// Retrieve as floating-point:
let as_f64: Option<f64> = gss.get_as(&["price"]);     // Some(19.95)
let int_as_f32: Option<f32> = gss.get_as(&["count"]); // Some(42.0)
```

### Allowing Key Redefinitions

By default, GSS rejects duplicate keys within the same scope. If your use-case requires overriding or cascading key definitions, configure `allow_redefinition`:

```rust
// Via parser options:
let gss = parse_str_with_options("key = 1, key = 2,", true)?;
assert_eq!(gss.get_as::<u32>(&["key"]), Some(2));

// On an Object instance:
let mut obj = Object::new().with_allow_redefinition(true);
obj.set_allow_redefinition(true);
assert!(obj.allow_redefinition());
```

### Cycle Detection & Depth Limiting

To prevent infinite loops when circular references exist (`a = b, b = a`), GSS limits reference traversal depth using `max_depth` (default is `20`). If the limit is exceeded, resolution returns `None`.

You can adjust this threshold:

```rust
let mut gss = parse_str("a = b, b = a,")?;
gss.set_max_depth(50); // Set custom recursion depth limit
```

### Inspection & Debugging

#### Iterating Fields
```rust
for field in gss.get_fields() {
    println!("Top-level key: {field}");
}
```

#### Dumping Structure
Pretty-prints the parsed AST hierarchy to stdout:
```rust
gss.dump(0);
```

---

## Type Mapping & Conversion Reference

| GSS Literal Syntax | Underlying Storage | `get::<T>()` (Exact `&T`) | `get_as::<T>()` (Converted `T`) |
| :--- | :--- | :--- | :--- |
| `100` / `0x64` | `u32` | `get::<u32>()` | `i8`..`i128`, `isize`, `u8`..`u128`, `usize`, `f32`, `f64` |
| `3.1415` | `f32` | `get::<f32>()` | `f32`, `f64`, integer types (if integer float) |
| `50%` | `Percent` (`f32`) | `get::<Percent>()` / `get::<f32>()` | `f32`, `f64` (evaluates as `0.5`) |
| `true` / `false` | `bool` | `get::<bool>()` | `bool` |
| `"sample"` | `String` | `get::<String>()` | `String` |
| `{ k = v }` | `Object` | `get::<Object>()` | — |

---

## Cargo Features

| Feature | Description |
| :--- | :--- |
| `interning` | Enables string interning in the underlying lexer via `lex-just-parse/interning`. |
| `internal-api` | Exposes internal parsing functions (`internal_parse`, `internal_parse_gss`, `internal_parse_object`, `internal_parse_value`, `internal_parse_object_from_str`) for extending or benchmarking the parser. |

Enable features in `Cargo.toml`:

```toml
[dependencies]
gss = { version = "0.1.0", features = ["interning"] }
```

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/) (2024 Edition supported, 1.85+ recommended)
- Cargo package manager

### Adding as a Dependency

```toml
[dependencies]
gss = { path = "path/to/gss" }
```

### Code Example

Create `src/main.rs`:

```rust
use gss::{parse_str, Percent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        theme = {
            primary_color = "#3498db",
            font_size = 16,
            opacity = 85%,
        },
        button = {
            background = theme.primary_color,
            height = theme.font_size,
        },
    "#;

    let gss = parse_str(source)?;

    let color: String = gss.get_as(&["button", "background"]).unwrap();
    let font_size: i32 = gss.get_as(&["button", "height"]).unwrap();
    let opacity: Percent = gss.get_as(&["theme", "opacity"]).unwrap();

    println!("Button Color: {color}");
    println!("Button Height: {font_size}px");
    println!("Opacity: {:.2}%", opacity * 100.0);

    Ok(())
}
```

---

## CLI / Example Binary

The repository includes a sample binary (`gss_test`):

```bash
cargo run --bin gss_test
```

This runs `src/bin/main.rs`, which parses `test/test3.gss` and pretty-prints the parsed hierarchy.

---

## Architecture

```
                 GSS Source String / File
                            │
                            ▼
              ┌───────────────────────────┐
              │  Lexer (lex-just-parse)   │
              └─────────────┬─────────────┘
                            │ Tokens
                            ▼
              ┌───────────────────────────┐
              │       Parser (GSS)        │
              └─────────────┬─────────────┘
                            │ Object AST
                            ▼
    ┌───────────────────────────────────────────────┐
    │              Object (HashMap)                 │
    │  - Value (Box<dyn Any>)                       │
    │  - Expr (Symbol, Access, RelAccess)           │
    │  - allow_redefinition / max_depth             │
    └───────────────────────┬───────────────────────┘
                            │ .get_as::<T>(&path) / .get::<T>(&path)
                            ▼
              ┌───────────────────────────┐
              │ Recursive Resolver Engine │
              │ (with max_depth cycles)   │
              └─────────────┬─────────────┘
                            │ FromGssValue / Downcast
                            ▼
                        Option<T>
```

- **Lexer & Parser**: Built using `lex-just-parse` combinators (`many1`, `sep_by`, `try_parse!`).
- **Storage**: `Object` encapsulates a `HashMap<String, Box<dyn Any + 'static>>`.
- **Expressions**: References are stored as `Expr` enums until evaluated at lookup time via `get_value_impl`.
- **Type Coercion**: Handled safely via the `FromGssValue` trait.

---

## Project Layout

```
.
├── Cargo.toml          # Rust package and dependency manifest
├── Cargo.lock          # Dependency lockfile
├── LICENSE             # MIT License
├── README.md           # Project documentation
├── CHANGELOG.md        # Version history
├── src/
│   ├── lib.rs          # Core library, parser, evaluation engine, and test suite
│   └── bin/
│       └── main.rs     # Sample runnable binary (gss_test)
└── test/
    ├── test.gss        # Sample stylesheet with primitives and percentages
    ├── test2.gss       # Sample stylesheet with absolute and relative references
    └── test3.gss       # Sample stylesheet with numeric literals (decimal, hex)
```

---

## Development & Testing

### Run Tests

```bash
cargo test
```

### Run Tests with All Features

```bash
cargo test --all-features
```

### Build in Release Mode

```bash
cargo build --release
```

---

## Troubleshooting & Common Pitfalls

### 1. `get::<T>()` returns `None` for existing keys
- **Cause**: Type mismatch with exact `Any` downcasting (e.g., requesting `&i32` or `&usize` when value is stored as `u32`).
- **Fix**: Use `.get_as::<T>()`, `.get_or::<T>()`, or `.get_or_default::<T>()` instead of `.get::<T>()`. These methods use the `FromGssValue` trait to automatically convert between common Rust numeric types (`i8`-`i128`, `isize`, `u8`-`u128`, `usize`, `f32`, `f64`).

### 2. Circular reference returns `None`
- **Cause**: Recursive reference exceeded `max_depth` (e.g. `a = b, b = a`).
- **Fix**: Check your GSS file for circular definitions or increase depth limit with `gss.set_max_depth(n)`.

### 3. Parse error: `Redefinition of key <key>`
- **Cause**: The same key was defined multiple times in the same scope, and strict mode is active.
- **Fix**: If duplicate keys should override previous definitions, parse using `parse_str_with_options(source, true)` or `load_gss_from_file_with_options(file_path, true)`.

---

## License

This project is licensed under the [MIT License](LICENSE).
