//! CLI integration tests — require `cargo build --release` before running.

use std::process::Command;

/// Returns the path to the release binary.
fn binary_path() -> String {
    format!("{}/target/release/bdpm-ingest", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn test_cli_help() {
    let bin = binary_path();
    let output = Command::new(&bin)
        .arg("--help")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run bdpm-ingest --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Command failed: stderr = {stderr}"
    );
    assert!(
        stdout.contains("BDPM drug database"),
        "Output missing 'BDPM drug database': {stdout}"
    );
    assert!(
        stdout.contains("import"),
        "Output missing 'import': {stdout}"
    );
}

#[test]
fn test_cli_stats() {
    let bin = binary_path();
    let output = Command::new(&bin)
        .args(["stats"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run bdpm-ingest stats");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Command failed: stderr = {stderr}"
    );
    assert!(
        stdout.contains("drugs:"),
        "Output missing 'drugs:': {stdout}"
    );
    // Extract drug count and verify > 0
    for line in stdout.lines() {
        if line.contains("drugs:") {
            let count_str = line.split(':').nth(1).unwrap_or("").trim();
            let count: i64 = count_str.parse().unwrap_or(0);
            assert!(count > 0, "Drug count should be > 0, got {count}");
            break;
        }
    }
}

#[test]
fn test_cli_logs() {
    let bin = binary_path();
    let output = Command::new(&bin)
        .args(["logs", "--limit", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run bdpm-ingest logs");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Command failed: stderr = {stderr}"
    );
    assert!(
        stdout.contains("file") && stdout.contains("rows"),
        "Output missing table header with 'file' and 'rows': {stdout}"
    );
}

#[test]
fn test_cli_dump_openapi() {
    let bin = binary_path();
    let output = Command::new(&bin)
        .args(["dump-open-api"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run bdpm-ingest dump-open-api");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Command failed: stderr = {stderr}"
    );
    assert!(
        stdout.starts_with("openapi:"),
        "Output should start with 'openapi:': {stdout}"
    );
}

#[test]
fn test_cli_stats_no_db() {
    let bin = binary_path();
    let output = Command::new(&bin)
        .args(["stats", "--data-dir", "/tmp/nonexistent_dir_xyz"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run bdpm-ingest stats with invalid data dir");

    assert!(
        !output.status.success(),
        "Command should fail when DB does not exist"
    );
}
