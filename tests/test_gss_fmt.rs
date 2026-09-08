use std::fs;
use std::process::Command;

#[test]
fn test_fmt_stdout_valid_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_gss-fmt"))
        .arg("tests/test3.gss")
        .output()
        .expect("failed to execute gss-fmt");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hex = 50,"));
    assert!(stdout.contains("key = 1,"));
    assert!(stdout.contains("other = 2,"));
}

#[test]
fn test_fmt_check_already_formatted_file() {
    let temp_path = "temp/test_fmt_already_formatted.gss";
    let formatted_content = "test = {\n    hex = 50,\n    key = 1,\n    other = 2,\n},\n";
    fs::write(temp_path, formatted_content).expect("failed to create temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_gss-fmt"))
        .args(["--check", temp_path])
        .output()
        .expect("failed to execute gss-fmt");

    let _ = fs::remove_file(temp_path);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
}

#[test]
fn test_fmt_check_unformatted_file() {
    let temp_path = "temp/test_fmt_unformatted.gss";
    let unformatted_content = "test={other=2,key=1,hex=0x32}";
    fs::write(temp_path, unformatted_content).expect("failed to create temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_gss-fmt"))
        .args(["-c", temp_path])
        .output()
        .expect("failed to execute gss-fmt");

    let _ = fs::remove_file(temp_path);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("File not formatted: temp/test_fmt_unformatted.gss"));
}

#[test]
fn test_fmt_write_in_place() {
    let temp_path = "temp/test_fmt_write.gss";
    let unformatted_content = "a={b=1,c=2}\nz=99";
    fs::write(temp_path, unformatted_content).expect("failed to create temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_gss-fmt"))
        .args(["-w", temp_path])
        .output()
        .expect("failed to execute gss-fmt");

    assert_eq!(output.status.code(), Some(0));

    let formatted_file_content =
        fs::read_to_string(temp_path).expect("failed to read formatted file");
    let _ = fs::remove_file(temp_path);

    let expected = "a = {\n    b = 1,\n    c = 2,\n},\nz = 99,\n";
    assert_eq!(formatted_file_content, expected);
}

#[test]
fn test_fmt_syntax_error() {
    let temp_path = "temp/test_fmt_syntax_err.gss";
    fs::write(temp_path, "invalid syntax here").expect("failed to create temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_gss-fmt"))
        .arg(temp_path)
        .output()
        .expect("failed to execute gss-fmt");

    let _ = fs::remove_file(temp_path);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty());
}

#[test]
fn test_fmt_nonexistent_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_gss-fmt"))
        .arg("non_existent_file_99999.gss")
        .output()
        .expect("failed to execute gss-fmt");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty());
}

#[test]
fn test_fmt_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_gss-fmt"))
        .arg("--help")
        .output()
        .expect("failed to execute gss-fmt");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: gss-fmt"));
    assert!(stdout.contains("--write"));
    assert!(stdout.contains("--check"));
}
