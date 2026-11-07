//! Build tools for the vulture mod. Run from anywhere in the repository:
//!
//! ```sh
//! cargo vulture model             # assets/models/vulture.bbmodel -> native/api/src/model.rs
//! cargo vulture sounds [folder]   # freesound downloads (default sounds-src/) -> content/assets/vulture/sounds/
//! cargo vulture pack              # native/ -> target/vulture.hnmod
//! cargo vulture install [instance] # builds both halves, installs them in the instance (default vulture-dev), builds it
//! ```
//!
//! `cargo vulture` is an alias (`.cargo/config.toml`) for `cargo run --release --manifest-path tools/Cargo.toml --`.
mod install;
mod model;
mod pack;
mod sounds;

use std::path::PathBuf;
use std::process::ExitCode;

/// The repository root: the parent of this crate's folder.
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tools/ is inside the repository")
        .to_path_buf()
}

pub type Result<T = ()> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest = args.get(1).map(String::as_str);
    let done = match args.first().map(String::as_str) {
        Some("model") => model::run(),
        Some("sounds") => sounds::run(rest),
        Some("pack") => pack::run().map(|_| ()),
        Some("install") => install::run(rest.unwrap_or("vulture-dev")),
        _ => Err("usage: cargo vulture model | sounds [folder] | pack | install [instance]".into()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
