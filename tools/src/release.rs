//! Prepares the published modules for Sunrise's registry (playsunrise.net/docs/publish-a-mod; the registry is
//! sunrise-org/sunrise-mods). A module is published when its `module.toml` has a `[registry]` table.
//!
//! For each one, `cargo creature release [module...]`:
//! - builds and checks its package (`build::module`) and copies it to `target/release/<id>-<version>.smod`, a version
//!   that is new (a published file never changes: a version already in its entry with another file is refused);
//! - signs it with minisign when the series has a key (`registry/minisign.pub`, the secret key's path in
//!   `$SUNRISE_MODS_KEY`), as `<file>.minisig`;
//! - writes its icon to `registry/icons/<id>.png` (`icon.rs`) and its entry to `registry/mods/<id>.toml`: its page
//!   (the manifest's name, authors and licence, `[registry]`'s summary, categories and tags, its README as the
//!   description) and its versions, the new one first with its changelog (its `CHANGELOG.md`'s section for it);
//! - prints what to do next: the GitHub release that holds the file, then the pull request to sunrise-mods.
//!
//! `registry/` mirrors the registry's own layout (`mods/<id>.toml`, `icons/<id>.png`): copy it into a fork of
//! sunrise-mods.
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::modules::{self, Module, Registry};

/// The repository the entries name as the source.
pub const SOURCE: &str = "https://github.com/EAnathos/sunrise-creatures";
/// Its files, for the README's images on a registry page.
const RAW: &str = "https://raw.githubusercontent.com/EAnathos/sunrise-creatures/main";
/// The GitHub logins allowed to publish updates.
const OWNERS: &[&str] = &["EAnathos"];
/// The Minecraft version.
const GAME: &str = "26.3";

/// A mod's categories (Sunrise's `sunrise_mod_package::index::page`).
pub const CATEGORIES: &[&str] = &[
    "adventure",
    "decoration",
    "economy",
    "equipment",
    "food",
    "game-mechanics",
    "library",
    "magic",
    "management",
    "minigame",
    "mobs",
    "optimization",
    "social",
    "storage",
    "technology",
    "transportation",
    "utility",
    "worldgen",
];

pub fn run(names: &[String]) -> crate::Result {
    crate::tab::run()?;
    let mut chosen = Vec::new();
    for m in std::iter::once(modules::library()?).chain(modules::mods()?) {
        if m.registry.is_some() && (names.is_empty() || names.contains(&m.id)) {
            chosen.push(m);
        }
    }
    if chosen.is_empty() {
        return Err("nothing to release: no module named has a [registry] table".into());
    }
    let out = crate::root().join("target/release");
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut released = Vec::new();
    for m in &chosen {
        released.push(release(m, &out)?);
    }
    println!("\n{}", next_steps(&released));
    Ok(())
}

/// One module's file and entry.
struct Released {
    id: String,
    version: String,
    file: PathBuf,
    sha256: String,
    /// Its changelog, for its GitHub release's notes.
    notes: PathBuf,
}

fn release(m: &Module, out: &Path) -> crate::Result<Released> {
    let registry = m.registry.as_ref().ok_or("no [registry]")?;
    let manifest = Manifest::read(m)?;
    let description = description(m)?;
    let problems = page_problems(registry, &description);
    if !problems.is_empty() {
        return Err(format!("{}: {}", m.id, problems.join("; ")));
    }
    let changelog = changelog(m, &manifest.version)?;
    let notes = out.join(format!("{}-{}.md", m.id, manifest.version));
    std::fs::write(&notes, &changelog).map_err(|e| format!("{}: {e}", notes.display()))?;
    let packed = crate::build::module(m)?;
    let name = format!("{}-{}.smod", m.id, manifest.version);
    let file = out.join(&name);
    std::fs::copy(&packed.smod, &file).map_err(|e| format!("{}: {e}", packed.smod.display()))?;
    let bytes = std::fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let sha256 = hex(&Sha256::digest(&bytes));
    let entry_path = crate::root().join(format!("registry/mods/{}.toml", m.id));
    let mut versions = Version::read_all(&entry_path)?;
    if let Some(old) = versions.iter().find(|v| v.version == manifest.version) {
        if old.sha256 != sha256 {
            return Err(format!(
                "{} {} is already published with another file: a published file never changes, raise the version",
                m.id, manifest.version
            ));
        }
    } else {
        let (key, signature) = sign(&file)?;
        versions.insert(
            0,
            Version {
                version: manifest.version.clone(),
                published: today(),
                game: GAME.into(),
                file: name,
                sha256: sha256.clone(),
                signature,
                changelog,
            },
        );
        crate::icon::make(
            &m.id,
            &registry.icon,
            &crate::root().join(format!("registry/icons/{}.png", m.id)),
        )?;
        let entry = entry(m, &manifest, registry, &description, key.as_deref(), &versions);
        if let Some(parent) = entry_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&entry_path, entry).map_err(|e| format!("{}: {e}", entry_path.display()))?;
    }
    println!("{}  {sha256}", file.display());
    Ok(Released {
        id: m.id.clone(),
        version: manifest.version,
        file,
        sha256,
        notes,
    })
}

/// What the release needs from the module's `sunrise-mod.toml`.
struct Manifest {
    name: String,
    version: String,
    authors: Vec<String>,
    license: String,
}

impl Manifest {
    fn read(m: &Module) -> crate::Result<Manifest> {
        let path = m
            .manifest()
            .ok_or_else(|| format!("{} has no sunrise-mod.toml", m.id))?;
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let t: toml::Table = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let text_of = |key: &str| {
            t.get(key)
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or_else(|| format!("{}: no {key}", path.display()))
        };
        let authors = t
            .get("authors")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        Ok(Manifest {
            name: text_of("name")?,
            version: text_of("version")?,
            authors,
            license: text_of("license")?,
        })
    }
}

/// The module's README as its registry page's description: its title left out (the page shows the name), its links
/// and images made absolute (to the repository on GitHub).
pub fn description(m: &Module) -> crate::Result<String> {
    let path = m.dir.join("README.md");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let body: Vec<&str> = text
        .lines()
        .skip_while(|l| l.starts_with("# ") || l.trim().is_empty())
        .collect();
    let dir = m.dir.strip_prefix(crate::root()).unwrap_or(&m.dir).to_path_buf();
    Ok(absolute_links(&body.join("\n"), &dir).trim().to_owned() + "\n")
}

/// `markdown` with its relative links (`[x](../../README.md)`) and images (`![](../../docs/models/a.png)`), relative
/// to `dir` in the repository, made absolute.
fn absolute_links(markdown: &str, dir: &Path) -> String {
    let mut out = String::new();
    let mut rest = markdown;
    while let Some(at) = rest.find("](") {
        let image = rest[..at].rfind('[').is_some_and(|i| rest[..i].ends_with('!'));
        out.push_str(&rest[..at + 2]);
        rest = &rest[at + 2..];
        let end = rest.find(')').unwrap_or(rest.len());
        let target = &rest[..end];
        if target.contains("://") || target.starts_with('#') || target.starts_with("mailto:") {
            out.push_str(target);
        } else {
            let path = normalise(&dir.join(target));
            let base = if image {
                RAW.to_owned()
            } else {
                format!("{SOURCE}/blob/main")
            };
            out.push_str(&format!("{base}/{path}"));
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// `path` with its `.` and `..` resolved, `/`-separated.
fn normalise(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::Normal(p) => parts.push(p.to_string_lossy().into_owned()),
            _ => {}
        }
    }
    parts.join("/")
}

/// What the registry would refuse on the page: the summary, categories, tags and the description's length.
pub fn page_problems(r: &Registry, description: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if r.summary.trim().is_empty() || r.summary.chars().count() > 160 {
        problems.push("the summary must be 1 to 160 characters".into());
    }
    let length = description.trim().chars().count();
    if !(50..=10_000).contains(&length) {
        problems.push(format!(
            "the description (its README) is {length} characters, not 50 to 10,000"
        ));
    }
    if r.categories.is_empty() || r.categories.len() > 3 {
        problems.push("1 to 3 categories".into());
    }
    for (k, c) in r.categories.iter().enumerate() {
        if !CATEGORIES.contains(&c.as_str()) {
            problems.push(format!("{c} is not a mod's category"));
        }
        if r.categories[..k].contains(c) {
            problems.push(format!("category {c} twice"));
        }
    }
    if r.tags.len() > 10 {
        problems.push("at most 10 tags".into());
    }
    for t in &r.tags {
        if t.is_empty() || t.chars().count() > 24 || t.chars().any(|c| c.is_uppercase()) {
            problems.push(format!("tag {t:?}: lowercase, 1 to 24 characters"));
        }
    }
    problems
}

/// The section of the module's `CHANGELOG.md` for `version` (`## 0.1.0`), without its heading.
pub fn changelog(m: &Module, version: &str) -> crate::Result<String> {
    let path = m.dir.join("CHANGELOG.md");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let heading = format!("## {version}");
    let start = text
        .lines()
        .position(|l| l.trim() == heading)
        .ok_or_else(|| format!("{}: no section {heading}", path.display()))?;
    let section: Vec<&str> = text
        .lines()
        .skip(start + 1)
        .take_while(|l| !l.starts_with("## "))
        .collect();
    Ok(section.join("\n").trim().to_owned() + "\n")
}

/// A published version, as its entry lists it.
struct Version {
    version: String,
    published: String,
    game: String,
    file: String,
    sha256: String,
    signature: Option<String>,
    changelog: String,
}

impl Version {
    /// The versions an entry already lists, newest first (none for a new project).
    fn read_all(entry: &Path) -> crate::Result<Vec<Version>> {
        if !entry.is_file() {
            return Ok(Vec::new());
        }
        let text = std::fs::read_to_string(entry).map_err(|e| format!("{}: {e}", entry.display()))?;
        let t: toml::Table = toml::from_str(&text).map_err(|e| format!("{}: {e}", entry.display()))?;
        let field = |v: &toml::Value, k: &str| v.get(k).and_then(|s| s.as_str()).unwrap_or_default().to_owned();
        Ok(t.get("versions")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .map(|v| Version {
                version: field(v, "version"),
                published: field(v, "published"),
                game: field(v, "game"),
                file: field(v, "file"),
                sha256: field(v, "sha256"),
                signature: v.get("signature").and_then(|s| s.as_str()).map(String::from),
                changelog: field(v, "changelog"),
            })
            .collect())
    }
}

/// Signs `file` with the series' minisign key when it has one: its public key (`registry/minisign.pub`) and the
/// signature's text. None without a public key.
fn sign(file: &Path) -> crate::Result<(Option<String>, Option<String>)> {
    let public = crate::root().join("registry/minisign.pub");
    if !public.is_file() {
        return Ok((None, None));
    }
    let key = std::fs::read_to_string(&public)
        .map_err(|e| format!("{}: {e}", public.display()))?
        .lines()
        .find(|l| !l.starts_with("untrusted comment:") && !l.trim().is_empty())
        .ok_or("registry/minisign.pub holds no key")?
        .trim()
        .to_owned();
    let secret = std::env::var("SUNRISE_MODS_KEY")
        .map_err(|_| "the series signs its files: set $SUNRISE_MODS_KEY to the minisign secret key's path")?;
    let path = file.to_string_lossy().into_owned();
    crate::install::run_in(&crate::root(), "minisign", &["-S", "-s", &secret, "-m", &path])?;
    let sig = PathBuf::from(format!("{path}.minisig"));
    let signature = std::fs::read_to_string(&sig).map_err(|e| format!("{}: {e}", sig.display()))?;
    Ok((Some(key), Some(signature)))
}

/// The registry entry (sunrise-mods' `mods/<id>.toml`).
fn entry(
    m: &Module,
    manifest: &Manifest,
    r: &Registry,
    description: &str,
    key: Option<&str>,
    versions: &[Version],
) -> String {
    let list = |items: &[String]| {
        let quoted: Vec<String> = items.iter().map(|s| basic(s)).collect();
        format!("[{}]", quoted.join(", "))
    };
    let owners: Vec<String> = OWNERS.iter().map(|s| s.to_string()).collect();
    let mut out = String::new();
    out.push_str("# Written by `cargo creature release` (tools/src/release.rs): edit the module, not this file.\n");
    out.push_str(&format!("id = {}\n", basic(&m.id)));
    out.push_str("kind = \"native-mod\"\n");
    out.push_str(&format!("name = {}\n", basic(&manifest.name)));
    out.push_str(&format!("summary = {}\n", basic(&r.summary)));
    out.push_str(&format!("description = {}\n", multiline(description)));
    out.push_str(&format!("authors = {}\n", list(&manifest.authors)));
    out.push_str(&format!("owners = {}\n", list(&owners)));
    out.push_str(&format!("license = {}\n", basic(&manifest.license)));
    out.push_str(&format!("source = {}\n", basic(SOURCE)));
    out.push_str(&format!("icon = {}\n", basic(&format!("icons/{}.png", m.id))));
    out.push_str(&format!("categories = {}\n", list(&r.categories)));
    out.push_str(&format!("tags = {}\n", list(&r.tags)));
    if let Some(key) = key {
        out.push_str(&format!("key = {}\n", basic(key)));
    }
    for v in versions {
        out.push_str("\n[[versions]]\n");
        out.push_str(&format!("version = {}\n", basic(&v.version)));
        out.push_str(&format!("published = {}\n", basic(&v.published)));
        out.push_str(&format!("game = {}\n", basic(&v.game)));
        out.push_str(&format!("file = {}\n", basic(&v.file)));
        out.push_str(&format!("sha256 = {}\n", basic(&v.sha256)));
        if let Some(sig) = &v.signature {
            out.push_str(&format!("signature = {}\n", multiline(sig)));
        }
        out.push_str(&format!("changelog = {}\n", multiline(&v.changelog)));
    }
    out
}

/// A one-line TOML string.
fn basic(s: &str) -> String {
    let escaped: String = s
        .chars()
        .flat_map(|c| match c {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            '\n' => vec!['\\', 'n'],
            c => vec![c],
        })
        .collect();
    format!("\"{escaped}\"")
}

/// A multi-line TOML string.
fn multiline(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace("\"\"\"", "\"\"\\\"");
    format!("\"\"\"\n{escaped}\"\"\"")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Today's date, `YYYY-MM-DD` (UTC).
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, m, d) = civil(secs as i64 / 86_400);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The calendar date of a day counted from 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// What to do with the files: one GitHub release per module version, then the registry pull request.
fn next_steps(released: &[Released]) -> String {
    let mut out = String::from("Next:\n1. Publish each file as a release of this repository (tag `<id>-<version>`):\n");
    for r in released {
        let sig = format!("{}.minisig", r.file.display());
        let extra = if Path::new(&sig).is_file() {
            format!(" {sig}")
        } else {
            String::new()
        };
        out.push_str(&format!(
            "   gh release create {id}-{v} {file}{extra} --title \"{id} {v}\" --notes-file {notes}\n",
            id = r.id,
            v = r.version,
            file = r.file.display(),
            notes = r.notes.display(),
        ));
    }
    out.push_str(
        "2. In a fork of sunrise-org/sunrise-mods, copy registry/mods/<id>.toml and registry/icons/<id>.png, commit with \
         `git commit -s`, and open a pull request linking each file and its SHA-256:\n",
    );
    for r in released {
        out.push_str(&format!(
            "   {SOURCE}/releases/download/{id}-{v}/{id}-{v}.smod  sha256 {sha}\n",
            id = r.id,
            v = r.version,
            sha = r.sha256
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_readme_s_relative_links_and_images_become_absolute() {
        let md = "![](../../docs/models/v-front.png) [series](../../README.md), [spec](SPECS.md), [web](https://x.y)";
        let out = absolute_links(md, Path::new("mods/vulture"));
        assert_eq!(
            out,
            format!(
                "![]({RAW}/docs/models/v-front.png) [series]({SOURCE}/blob/main/README.md), \
                 [spec]({SOURCE}/blob/main/mods/vulture/SPECS.md), [web](https://x.y)"
            )
        );
    }

    #[test]
    fn days_are_calendar_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_775), (2026, 11, 18));
        assert_eq!(civil(11_016), (2000, 2, 29));
    }

    #[test]
    fn toml_strings_are_escaped() {
        assert_eq!(basic("a \"b\" \\c"), "\"a \\\"b\\\" \\\\c\"");
        assert_eq!(multiline("x \"\"\" y\n"), "\"\"\"\nx \"\"\\\" y\n\"\"\"");
    }

    #[test]
    fn a_page_is_checked_as_the_registry_does() {
        let r = Registry {
            summary: "A vulture.".into(),
            categories: vec!["mobs".into(), "bloom".into()],
            tags: vec!["Vulture".into()],
            icon: "vulture".into(),
        };
        let problems = page_problems(&r, "short");
        assert_eq!(problems.len(), 3, "{problems:?}");
    }
}
