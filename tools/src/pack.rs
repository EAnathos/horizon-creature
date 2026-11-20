//! Packages a module into a Sunrise `.smod` (Sunrise's modding book, "The manifest and the package").
//!
//! A module's root holds its package (`sunrise-mod.toml`, `Cargo.toml`, `src/`, `assets/`, `data/`, `README.md`) beside
//! files that are the repository's (`module.toml`, `SPECS.md`, `sources/`), which a package refuses. So the package's
//! files are first copied to `target/stage/<kind>/<id>/` (`stage`), then
//! zipped into `target/<id>.smod`. A module whose Rust isn't ported yet (`module.toml`'s `ported`) is staged as content
//! only: no `Cargo.toml`, `src/` or `[native]`.
use std::io::Write as _;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

use crate::modules::Module;

/// What packaging a module made: its `.smod` (its staged folder is `stage_dir`).
pub struct Packed {
    pub smod: PathBuf,
}

/// Stages and zips the module.
pub fn run(module: &Module) -> crate::Result<Packed> {
    let stage = stage(module)?;
    let smod = crate::root().join(format!("target/{}.smod", module.id));
    zip_dir(&stage, &smod)?;
    println!("{}", smod.display());
    Ok(Packed { smod })
}

/// The folder of the module's staged package: `target/stage/native/<id>/` for a module with Rust,
/// `target/stage/content/<id>/` for content only.
pub fn stage_dir(module: &Module) -> PathBuf {
    let kind = if module.native().is_some() { "native" } else { "content" };
    crate::root().join("target/stage").join(kind).join(&module.id)
}

/// Copies the module's package files to `target/stage/<id>/` (emptied first) and returns it.
pub fn stage(module: &Module) -> crate::Result<PathBuf> {
    let manifest_path = module
        .manifest()
        .ok_or_else(|| format!("{} has no sunrise-mod.toml", module.id))?;
    let manifest = std::fs::read_to_string(&manifest_path).map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    let id = toml_value(&manifest, "id").ok_or("sunrise-mod.toml has no id")?;
    if id != module.id {
        return Err(format!(
            "{}: id {id}, but module.toml says {}",
            manifest_path.display(),
            module.id
        ));
    }
    let out = stage_dir(module);
    if out.exists() {
        std::fs::remove_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let native = module.native().is_some();
    let manifest = if native { manifest } else { content_only(&manifest) };
    std::fs::write(out.join("sunrise-mod.toml"), manifest).map_err(|e| e.to_string())?;
    let mut parts = vec!["assets", "data", "README.md", "icon.png"];
    if native {
        parts.extend(["Cargo.toml", "src"]);
    }
    for part in parts {
        let from = module.dir.join(part);
        if from.exists() {
            copy(&from, &out.join(part))?;
        }
    }
    copy(&crate::root().join("LICENSE"), &out.join("LICENSE"))?;
    // A published module's icon, for the game's Mods screen.
    if let Some(registry) = &module.registry {
        crate::icon::make(&module.id, &registry.icon, &out.join("icon.png"))?;
    }
    Ok(out)
}

/// The manifest without its `[native]` table: a module packaged as content only.
fn content_only(manifest: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            skipping = t == "[native]";
        }
        if !skipping {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn copy(from: &Path, to: &Path) -> crate::Result {
    if from.is_dir() {
        std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
        for entry in std::fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if let Some(name) = path.file_name() {
                copy(&path, &to.join(name))?;
            }
        }
        Ok(())
    } else {
        std::fs::copy(from, to)
            .map(drop)
            .map_err(|e| format!("{}: {e}", from.display()))
    }
}

/// Zips `dir`'s files (sorted, with a fixed date, so the same sources always make the same file) into `out`.
fn zip_dir(dir: &Path, out: &Path) -> crate::Result {
    let mut files = Vec::new();
    collect(dir, &mut files)?;
    files.sort();
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = std::fs::File::create(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut zip = ZipWriter::new(file);
    let date = DateTime::from_date_and_time(2026, 1, 1, 0, 0, 0).map_err(|e| e.to_string())?;
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(date);
    for path in &files {
        let rel = path.strip_prefix(dir).map_err(|e| e.to_string())?;
        let name = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

/// The files under `dir`.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> crate::Result {
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

/// The first `key = "value"` line's value.
pub fn toml_value(toml: &str, key: &str) -> Option<String> {
    toml.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(key)?.trim_start().strip_prefix('=')?.trim();
        Some(rest.strip_prefix('"')?.strip_suffix('"')?.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::{content_only, toml_value};

    #[test]
    fn reads_the_first_matching_line() {
        let toml = "id = \"vulture\"\nversion = \"0.0.1\"\n[native]\nsunrise = \"abc\"\n";
        assert_eq!(toml_value(toml, "version").as_deref(), Some("0.0.1"));
        assert_eq!(toml_value(toml, "sunrise").as_deref(), Some("abc"));
        assert_eq!(toml_value(toml, "missing"), None);
    }

    #[test]
    fn a_content_only_manifest_loses_its_native_table_only() {
        let toml = "id = \"x\"\n[native]\ngame = \"26.3\"\n\n[dependencies]\ncreature = \"^0.1\"\n";
        let out = content_only(toml);
        assert!(!out.contains("[native]") && !out.contains("game"));
        assert!(out.contains("[dependencies]\ncreature"));
    }
}
