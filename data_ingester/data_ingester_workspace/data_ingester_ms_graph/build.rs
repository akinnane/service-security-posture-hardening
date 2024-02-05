// build.rs
use std::process::Command;

fn main() {
    // note: add error checking yourself.
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("Should be able to get current GIT commit hash");

    let git_hash = String::from_utf8(output.stdout).expect("Git hash should be valid UTF8");
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // Check TOML is valid
    let contents = include_str!("ms_graph.toml");
    let _decoded: toml::Table = toml::from_str(contents).expect("Toml should be valid");
}
