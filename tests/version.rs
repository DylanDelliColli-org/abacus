use std::process::Command;

#[test]
fn long_version_flag_prints_the_crate_version_and_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .arg("--version")
        .output()
        .expect("abacus binary should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "stdout: {stdout}"
    );
}

#[test]
fn short_version_flag_prints_the_crate_version_and_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .arg("-V")
        .output()
        .expect("abacus binary should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "stdout: {stdout}"
    );
}
