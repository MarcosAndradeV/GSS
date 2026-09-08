use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("Usage: gss-check <file.gss>");
        process::exit(1);
    }

    if args.len() == 1 && (args[0] == "-h" || args[0] == "--help") {
        println!("Usage: gss-check <file.gss>...");
        process::exit(0);
    }

    let mut has_error = false;

    for file in &args {
        if let Err(err) = gss::load_gss_from_file(file) {
            eprintln!("{err}");
            has_error = true;
        }
    }

    if has_error {
        process::exit(1);
    }
}
