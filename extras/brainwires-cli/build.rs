use chrono::Utc;

fn main() {
    let build_time = Utc::now();
    let build_timestamp = build_time.to_rfc3339();
    // Human-readable date for the version string: "2026-03-30 UTC"
    let build_date = build_time.format("%Y-%m-%d UTC").to_string();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_timestamp);

    let git_hash = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    if let Some(ref hash) = git_hash {
        println!("cargo:rustc-env=GIT_HASH={}", hash);
    }

    let pkg_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let full_version = match &git_hash {
        Some(hash) => format!("{} (built {} • {})", pkg_version, build_date, hash),
        None => format!("{} (built {})", pkg_version, build_date),
    };
    println!("cargo:rustc-env=FULL_VERSION={}", full_version);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
}
