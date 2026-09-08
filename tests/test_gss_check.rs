use std::fs;
use std::process::Command;

#[test]
fn test_check_valid_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_gss-check"))
        .arg("tests/test.gss")
        .output()
        .expect("failed to execute gss-check");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
}

#[test]
fn test_check_invalid_syntax() {
    let temp_path = "temp/test_syntax_err.gss";
    fs::write(temp_path, "key 123,").expect("failed to create temp file");

    let output = Command::new(env!("CARGO_BIN_EXE_gss-check"))
        .arg(temp_path)
        .output()
        .expect("failed to execute gss-check");

    let _ = fs::remove_file(temp_path);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty());
    assert!(stderr.contains("Expect = got Number(D)"));
}

#[test]
fn test_check_nonexistent_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_gss-check"))
        .arg("non_existent_file_12345.gss")
        .output()
        .expect("failed to execute gss-check");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty());
}

#[test]
fn test_check_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_gss-check"))
        .output()
        .expect("failed to execute gss-check");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage:"));
}
