fn main() {
    // Try in order: RAILWAY_GIT_COMMIT_SHA, GIT_HASH env var, git command
    let git_hash = std::env::var("RAILWAY_GIT_COMMIT_SHA")
        .map(|s| s[..7.min(s.len())].to_string())
        .or_else(|_| std::env::var("GIT_HASH"))
        .unwrap_or_else(|_| {
            std::process::Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .unwrap_or_else(|| "unknown".to_string())
                .trim()
                .to_string()
        });

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!(
        "cargo:rustc-env=BUILD_TIME={}",
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
    );
}
