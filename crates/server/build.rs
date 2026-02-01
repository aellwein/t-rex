use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use vergen_gitcl::{Emitter, Gitcl};

/// Where the web app is built to. A separate target directory keeps the nested
/// build from waiting on the lock held by the outer `cargo build`.
const WEBAPP_TARGET_DIR: &str = "target/webapp-build";

/// Where `dx bundle` writes the static files that `embed.rs` embeds.
const WEBAPP_PUBLIC_DIR: &str = "target/webapp-build/dx/webapp/release/web/public";

fn main() -> Result<(), std::io::Error> {
    let gitcl = Gitcl::all_git();
    Emitter::default()
        .add_instructions(&gitcl)
        .expect("failed to add instructions")
        .emit()
        .expect("failed emitting git information");

    println!("cargo:rerun-if-env-changed=T_REX_SKIP_WEBAPP");
    // Only the `embed` feature (the default) serves the web app from the binary.
    if env::var_os("CARGO_FEATURE_EMBED").is_some() && env::var_os("T_REX_SKIP_WEBAPP").is_none() {
        build_webapp();
    }
    Ok(())
}

fn build_webapp() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    // The workspace root is two levels up: crates/server -> crates -> root.
    let root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("crates/server/ has a workspace root two levels up")
        .to_path_buf();
    let webapp = root.join("crates").join("webapp");

    for path in [
        "crates/webapp/src",
        "crates/webapp/i18n",
        "crates/webapp/assets",
        "crates/webapp/Cargo.toml",
        "crates/webapp/Dioxus.toml",
        "crates/webapp/tailwind.css",
        "crates/dto/src",
        "crates/dto/Cargo.toml",
    ] {
        let full = root.join(path);
        // A missing path would make cargo rerun this script on every build.
        if full.exists() {
            println!("cargo:rerun-if-changed={}", full.display());
        }
    }

    let status = Command::new("dx")
        .args(["bundle", "--web", "--release"])
        .current_dir(&webapp)
        .env("CARGO_TARGET_DIR", root.join(WEBAPP_TARGET_DIR))
        // The server build sets RUSTFLAGS for musl static linking; the wasm build must not inherit it.
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .status()
        .unwrap_or_else(|err| {
            panic!(
                "could not run `dx` to build the web app ({err}). \
                 Install the Dioxus CLI with `cargo install dioxus-cli`, \
                 or set T_REX_SKIP_WEBAPP=1 to reuse the last build."
            )
        });
    if !status.success() {
        panic!("`dx bundle --web --release` failed in {}", webapp.display());
    }

    let public = root.join(WEBAPP_PUBLIC_DIR);
    if !Path::new(&public.join("index.html")).exists() {
        panic!(
            "the web app build did not produce {}; check the dx output layout",
            public.join("index.html").display()
        );
    }
}
