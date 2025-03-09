use std::process::Command;

fn main() {
    let git_commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    println!(
        "cargo:rustc-env=DEVSPACE_FULL_VERSION={} ({})",
        env!("CARGO_PKG_VERSION"),
        git_commit
    );
}
