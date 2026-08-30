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
  - [Cycle Detection & Depth Limiting](#cycle-detection--depth-limiting)
  - [Inspection & Debugging](#inspection--debugging)
- [Type Mapping](#type-mapping)
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

**Graph Style Sheets (GSS)** provides a simple yet powerful declarative format for configuring hierarchical and graph-like structures. Unlike static serialization formats like JSON or TOML, GSS natively supports **dynamic symbol resolution**, **absolute and relative property references**, **percentages**, and **cycle protection**.

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
- **Safety & Validation**: Duplicate key detection at parse time prevents accidental overrides.
- **Ergonomic Rust API**: Type-safe downcasting using generic getters (`.get::<T>()`, `.get_or()`, `.get_or_default()`).

---

## GSS Language Syntax

GSS files consist of top-level key-value assignments:

```gss
key = value,
```

Trailing and separating commas are optional in object definitions.

### Value Types

| Type | Syntax Example | Rust Downcast Type | Description |
| :--- | :--- | :--- | :--- |
| **Integer** | `count = 42,`<br>`hex = 0x32,` | `u32` | Standard integer with radix support (decimal, hex, binary, octal). |
| **Float** | `price = 42.12,`<br>`scale = 0.5,` | `f32` | Single-precision IEEE 754 float. |
| **Percentage** | `width = 89%,` | `Percent` (`f32`) | Automatically parsed as fractional float (`0.89`). |
| **Boolean** | `visible = true,`<br>`debug = false,` | `bool` | Boolean literal. |
| **String** | `title = "Hello World",` | `String` | Quoted string literal with escape sequence support. |
| **Object** | `nested = { ... }` | `Object` / `Gss` | Nested key-value dictionary. |

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
use gss::{load_gss_from_file, parse_str, Gss};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Parse directly from a string
    let gss_from_str: Gss = parse_str(r#"
        window = {
            width = 800,
            height = 600,
        }
    "#)?;

    // Or load from a file
    let gss_from_file: Gss = load_gss_from_file("config.gss")?;

    Ok(())
}
```

### Querying Values

Query values by providing a path of string slices (`&[&str]`):

```rust
// 1. Get an Option<&T> (returns None on missing key or type mismatch)
if let Some(width) = gss.get::<u32>(&["window", "width"]) {
    println!("Width: {width}");
}

// 2. Get cloned value with fallback
let height = gss.get_or::<u32>(&["window", "height"], 480);

// 3. Get cloned value with Default::default() fallback
let title = gss.get_or_default::<String>(&["window", "title"]); // ""
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

## Type Mapping

When querying values via `.get::<T>()`, use the corresponding Rust type:

| GSS Syntax | Stored Representation | Queried via `get::<T>()` |
| :--- | :--- | :--- |
| `100` / `0x64` | `u32` | `gss.get::<u32>(&[...])` |
| `3.1415` | `f32` | `gss.get::<f32>(&[...])` |
| `50%` | `f32` (divided by 100.0) | `gss.get::<Percent>(&[...])` or `gss.get::<f32>(&[...])` |
| `true` / `false` | `bool` | `gss.get::<bool>(&[...])` |
| `"sample"` | `String` | `gss.get::<String>(&[...])` |
| `{ k = v }` | `Object` | `gss.get::<Object>(&[...])` |

> [!NOTE]
> Integers are stored as `u32` and floats as `f32`. Querying with `i32`, `i64`, or `f64` will return `None` due to `Any` downcasting.

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

    let color: &String = gss.get(&["button", "background"]).unwrap();
    let font_size: &u32 = gss.get(&["button", "height"]).unwrap();
    let opacity: &Percent = gss.get(&["theme", "opacity"]).unwrap();

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
    └───────────────────────┬───────────────────────┘
                            │ .get::<T>(&path)
                            ▼
              ┌───────────────────────────┐
              │ Recursive Resolver Engine │
              │ (with max_depth cycles)   │
              └─────────────┬─────────────┘
                            │ Downcast
                            ▼
                       Option<&T>
```

- **Lexer & Parser**: Built using `lex-just-parse` combinators (`many1`, `sep_by`, `try_parse!`).
- **Storage**: `Object` encapsulates a `HashMap<String, Box<dyn Any + 'static>>`.
- **Expressions**: References are stored as `Expr` enums until evaluated at lookup time via `get_impl`.

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
- **Cause**: Type mismatch in `downcast_ref::<T>()`.
- **Fix**: Verify the Rust type mapping. Integers are `u32` (not `i32`/`usize`), floats are `f32` (not `f64`).

### 2. Circular reference returns `None`
- **Cause**: Recursive reference exceeded `max_depth` (e.g. `a = b, b = a`).
- **Fix**: Check your GSS file for circular definitions or increase depth limit with `gss.set_max_depth(n)`.

### 3. Parse error: `Redefinition of key <key>`
- **Cause**: The same key was defined multiple times in the same scope.
- **Fix**: Ensure keys within an object or at root level are unique.

---

## License

This project is licensed under the [MIT License](LICENSE).
