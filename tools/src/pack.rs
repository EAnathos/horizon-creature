//! Packages a module's native part (`common/native/` or `mods/<mod>/native/`) into `target/<id>.hnmod` (Horizon's
//! docs/api/mod-files.md).
//!
//! The `.hnmod` holds `horizon-native.json` and `mixins/<id>/` (the bundle as is, without `target/`). It is what the
//! launcher installs in `horizon-native-mods/` and builds: a bundle built from its folder is not "installed", and the
//! launcher's Play button ignores it.
use std::io::Write as _;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

/// horizon-mixins of the pinned build (the `native-mods.json` of the launcher's builds).
const MIXINS: &str = "5a64af938ec86b00d53d16a408aabc0038fb3e57";

/// Builds the module's `.hnmod` and returns its path.
pub fn run(module: &crate::modules::Module) -> crate::Result<PathBuf> {
    let root = crate::root();
    let bundle = module
        .native()
        .ok_or_else(|| format!("{} has no native part", module.id))?;
    let manifest_path = bundle.join("horizon-mixin.toml");
    let toml = std::fs::read_to_string(&manifest_path).map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    let id = toml_value(&toml, "id").ok_or("horizon-mixin.toml has no id")?;
    if id != module.id {
        return Err(format!(
            "{}: id {id}, but module.toml says {}",
            manifest_path.display(),
            module.id
        ));
    }
    let out = root.join(format!("target/{id}.hnmod"));
    let version = toml_value(&toml, "version").ok_or("horizon-mixin.toml has no version")?;
    let horizon = toml_value(&toml, "horizon_revision").ok_or("horizon-mixin.toml pins no horizon_revision")?;
    let manifest = format!(
        "{{\n  \"game\": \"26.3\",\n  \"horizon\": \"{horizon}\",\n  \"id\": \"{id}\",\n  \"mixins\": \"{MIXINS}\",\n  \
         \"schema\": 1,\n  \"version\": \"{version}\"\n}}\n"
    );
    let mut files = Vec::new();
    collect(&bundle, &mut files)?;
    files.sort();

    std::fs::create_dir_all(out.parent().unwrap_or(&root)).map_err(|e| e.to_string())?;
    let file = std::fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut zip = ZipWriter::new(file);
    // A fixed date, so the same sources always make the same file.
    let date = DateTime::from_date_and_time(2026, 1, 1, 0, 0, 0).map_err(|e| e.to_string())?;
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(date);
    let mut add = |name: String, bytes: &[u8]| -> crate::Result {
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        zip.write_all(bytes).map_err(|e| e.to_string())
    };
    add("horizon-native.json".into(), manifest.as_bytes())?;
    for path in &files {
        let rel = path.strip_prefix(&bundle).map_err(|e| e.to_string())?;
        let rel = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        add(format!("mixins/{id}/{rel}"), &bytes)?;
    }
    zip.finish().map_err(|e| e.to_string())?;
    println!("{}: {} files, version {version}", out.display(), files.len());
    Ok(out)
}

/// The `.rs` and `.toml` files under `dir`, skipping `target/`.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> crate::Result {
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "target") {
                collect(&path, out)?;
            }
        } else if path.extension().is_some_and(|x| x == "rs" || x == "toml") {
            out.push(path);
        }
    }
    Ok(())
}

/// The first `key = "value"` line's value.
fn toml_value(toml: &str, key: &str) -> Option<String> {
    toml.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(key)?.trim_start().strip_prefix('=')?.trim();
        Some(rest.strip_prefix('"')?.strip_suffix('"')?.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::toml_value;

    #[test]
    fn reads_the_first_matching_line() {
        let toml = "[mod]\nid = \"vulture\"\nversion = \"0.0.1\"\n[compatibility]\nhorizon_revision = \"abc\"\n";
        assert_eq!(toml_value(toml, "version").as_deref(), Some("0.0.1"));
        assert_eq!(toml_value(toml, "horizon_revision").as_deref(), Some("abc"));
        assert_eq!(toml_value(toml, "missing"), None);
    }
}
