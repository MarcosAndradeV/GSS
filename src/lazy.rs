
#[macro_export]
macro_rules! lazy_gss {
    (
        $(#[$meta:meta])*
        $vis:vis static $name:ident = $file_path:literal;

        $(
            $(#[$fn_meta:meta])*
            $fn_vis:vis $fn_name:ident() -> $ty:ty = [$($key:literal),+ $(,)?] or_else $default:expr;
        )*
    ) => {
        $(#[$meta])*
        $vis static $name: std::sync::LazyLock<std::sync::Mutex<$crate::Object>> =
            std::sync::LazyLock::new(|| std::sync::Mutex::new(
                match $crate::load_gss_from_file($file_path) {
                    Ok(obj) => obj,
                    Err(err) => {
                        eprintln!("Cannot load `{}`: {}", $file_path, err);
                        std::process::exit(1);
                    }
                }
            ));

        /// Executes a closure against the locked gss object.
        $vis fn with<R>(f: impl FnOnce(&mut $crate::Object) -> R) -> R {
            let mut guard = $name.lock().unwrap_or_else(|e| e.into_inner());
            f(&mut guard)
        }

        $(
            $(#[$fn_meta])*
            $fn_vis fn $fn_name() -> $ty {
                with(|setup| setup.get_or(&[$($key),+], $default))
            }
        )*
    };
}
