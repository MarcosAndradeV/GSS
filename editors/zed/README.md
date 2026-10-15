# GSS Extension for Zed

Official [Zed](https://zed.dev/) extension providing rich language support for **Graph Style Sheets (GSS)**.

## Features

- **Tree-sitter Syntax Highlighting**: Highlighting for keys, identifiers, numbers (decimal & hexadecimal `0x...`), floating-point values, percentages (`89%`), booleans (`true`/`false`), strings with escape sequences, path access expressions (`style.image.top`, `.width`), and comments (`//`, `/* */`).
- **Bracket Matching & Autoclosing**: Autoclosing and pair matching for `{}` and `[]`.
- **Indentation & Formatting**:
  - Automatic indentation for nested object blocks and lists.
  - Automatic on-save and on-demand document formatting powered by [`gss-fmt`](file:///home/marcos/Projects/gss/src/bin/gss-fmt.rs) through [`gss-lsp`](file:///home/marcos/Projects/gss/src/bin/gss-lsp.rs).
- **Live Syntax Diagnostics**: Immediate error reporting with line/column pointers directly in the editor buffer.

---

## Installation & Local Development

### 1. Build the GSS Language Server
Ensure `gss-lsp` is compiled in your workspace or installed globally on `$PATH`:

```bash
# Build in workspace target directory (auto-discovered by the extension)
cargo build --bin gss-lsp

# Or install globally onto your PATH
cargo install --path . --bin gss-lsp
```

### 2. Install as a Dev Extension in Zed
1. In Zed, open the Command Palette (`Ctrl-Shift-P` on Linux/Windows, `Cmd-Shift-P` on macOS).
2. Type and select `zed: install dev extension`.
3. Select this directory: `/path/to/gss/editors/zed`.
4. Open any `.gss` file. Zed will now provide syntax highlighting, diagnostics, and auto-formatting!

---

## Formatting Configuration

The extension automatically manages `gss-lsp`. If you prefer to invoke `gss-fmt` directly as an external command without the language server, add this to your Zed `settings.json`:

```json
{
  "languages": {
    "GSS": {
      "formatter": {
        "external": {
          "command": "gss-fmt",
          "arguments": ["-"]
        }
      }
    }
  }
}
```
