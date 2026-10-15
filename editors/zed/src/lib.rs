use zed_extension_api::{self as zed, Command, LanguageServerId, Result, Worktree};

struct GssExtension;

impl zed::Extension for GssExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let binary_path = worktree
            .which("gss-lsp")
            .or_else(|| {
                let candidate_release = format!("{}/target/release/gss-lsp", worktree.root_path());
                if std::path::Path::new(&candidate_release).exists() {
                    Some(candidate_release)
                } else {
                    let candidate_debug = format!("{}/target/debug/gss-lsp", worktree.root_path());
                    if std::path::Path::new(&candidate_debug).exists() {
                        Some(candidate_debug)
                    } else {
                        None
                    }
                }
            })
            .ok_or_else(|| {
                "gss-lsp binary not found in PATH or workspace target/. Run `cargo build --bin gss-lsp` or `cargo install --path .`".to_string()
            })?;

        Ok(Command {
            command: binary_path,
            args: vec![],
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(GssExtension);
