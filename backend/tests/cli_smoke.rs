use std::process::Command;

#[test]
fn help_displays_overview() {
    let binary = env!("CARGO_BIN_EXE_rust-market-data");
    let output = Command::new(binary)
        .arg("--help")
        .output()
        .expect("invoke rust-market-data --help");

    assert!(output.status.success(), "help command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Correlated market data simulator"),
        "expected overview text in help output"
    );
}
