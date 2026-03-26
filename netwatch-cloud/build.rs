fn main() {
    // Try in order: RAILWAY_GIT_COMMIT_SHA, GIT_HASH env, .git-hash file, git command
    let git_hash = std::env::var("RAILWAY_GIT_COMMIT_SHA")
        .map(|s| s[..7.min(s.len())].to_string())
        .or_else(|_| std::env::var("GIT_HASH"))
        .or_else(|_| {
            // .git-hash is written by the Dockerfile before cargo build
            std::fs::read_to_string(
                std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(".git-hash"),
            )
            .map(|s| s.trim().to_string())
        })
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
