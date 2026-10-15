use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::Path;
use std::process;

fn write_stdout(text: &str) {
    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(stdout.lock());
    let _ = handle.write_all(text.as_bytes());
}

fn print_usage() {
    println!(
        "Usage: gss-fmt [OPTIONS] <file.gss>...\n\n\
        Options:\n  \
        -w, --write, -i, --in-place   Write formatted result to source file(s) in place\n  \
        -c, --check                   Check if file(s) are formatted without modifying them\n  \
        --allow-redefinition          Allow duplicate key redefinitions during parsing\n  \
        -h, --help                    Display this help message"
    );
}

pub fn run() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        // If stdin is a terminal, show usage; otherwise read and format stdin
        if io::stdin().is_terminal() {
            eprintln!("Usage: gss-fmt [OPTIONS] <file.gss>...");
            eprintln!("For help, run: gss-fmt --help");
            process::exit(1);
        } else {
            let mut buffer = String::new();
            if let Err(e) = io::stdin().read_to_string(&mut buffer) {
                eprintln!("Error reading stdin: {e}");
                process::exit(1);
            }
            match gss::format_str(&buffer) {
                Ok(formatted) => {
                    write_stdout(&formatted);
                    process::exit(0);
                }
                Err(err) => {
                    eprintln!("{err}");
                    process::exit(1);
                }
            }
        }
    }

    let mut write_in_place = false;
    let mut check_only = false;
    let mut allow_redefinition = false;
    let mut files = Vec::new();

    for arg in &args {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            "-w" | "--write" | "-i" | "--in-place" => {
                write_in_place = true;
            }
            "-c" | "--check" => {
                check_only = true;
            }
            "--allow-redefinition" => {
                allow_redefinition = true;
            }
            "-" => {
                files.push("-".to_string());
            }
            other if other.starts_with('-') => {
                eprintln!("Unknown option: {other}");
                eprintln!("For help, run: gss-fmt --help");
                process::exit(1);
            }
            file => {
                files.push(file.to_string());
            }
        }
    }

    if files.is_empty() {
        if io::stdin().is_terminal() {
            eprintln!("Error: No input files specified.");
            eprintln!("Usage: gss-fmt [OPTIONS] <file.gss>...");
            process::exit(1);
        } else {
            files.push("-".to_string());
        }
    }

    let mut has_error = false;
    let mut unformatted_files = Vec::new();

    for file in &files {
        if file == "-" {
            let mut buffer = String::new();
            if let Err(e) = io::stdin().read_to_string(&mut buffer) {
                eprintln!("Error reading stdin: {e}");
                has_error = true;
                continue;
            }
            match gss::format_str_with_options(&buffer, allow_redefinition) {
                Ok(formatted) => {
                    if check_only {
                        if formatted != buffer {
                            unformatted_files.push("<stdin>".to_string());
                        }
                    } else {
                        write_stdout(&formatted);
                    }
                }
                Err(err) => {
                    eprintln!("stdin: {err}");
                    has_error = true;
                }
            }
            continue;
        }

        let path = Path::new(file);
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(err) => {
                eprintln!("{file}: {err}");
                has_error = true;
                continue;
            }
        };

        match gss::format_str_with_options(&content, allow_redefinition) {
            Ok(formatted) => {
                if check_only {
                    if formatted != content {
                        unformatted_files.push(file.clone());
                    }
                } else if write_in_place {
                    if formatted != content {
                        if let Err(err) = fs::write(path, &formatted) {
                            eprintln!("Error writing {file}: {err}");
                            has_error = true;
                        }
                    }
                } else {
                    write_stdout(&formatted);
                }
            }
            Err(err) => {
                eprintln!("{file}: {err}");
                has_error = true;
            }
        }
    }

    if check_only {
        if !unformatted_files.is_empty() {
            for unformatted in &unformatted_files {
                eprintln!("File not formatted: {unformatted}");
            }
            process::exit(1);
        }
    }

    if has_error {
        process::exit(1);
    }
}

#[allow(dead_code)]
fn main() {
    run();
}
