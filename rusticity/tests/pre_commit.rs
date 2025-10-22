use std::process::Command;

#[test]
fn test_formatting() {
    let output = Command::new("cargo")
        .args(["fmt", "--all", "--check"])
        .output()
        .expect("Failed to run cargo fmt");

    assert!(
        output.status.success(),
        "Code is not formatted. Run: cargo fmt --all\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_clippy() {
    let output = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .output()
        .expect("Failed to run cargo clippy");

    assert!(
        output.status.success(),
        "Clippy found issues:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
