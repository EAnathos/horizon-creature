//! Makes a mod's sounds from freesound downloads, as its `assets/sounds.toml` says: its sources (freesound numbers) and
//! its clips (a slice of a source, pitch, fade, gain). The sources are CC0 (public domain: nothing to credit).
//!
//! The downloaded files keep the name freesound gives them (`740195__el_boss__goose-hiss.wav`): they are found by
//! their number, in `sounds-src/` or the folder given. Output: `content/assets/<mod>/<out>/*.ogg` (`out` from the
//! file, `sounds/<mod>` by default), mono 44.1 kHz, normalized volume, like vanilla sounds. Needs ffmpeg.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;

/// A mod's `assets/sounds.toml`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    out: Option<String>,
    sources: BTreeMap<String, u32>,
    clip: Vec<Clip>,
}

/// One clip: a slice of a source file, resampled by `pitch` (slower and lower below 1).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Clip {
    name: String,
    source: String,
    start: f64,
    /// Output length in seconds; `None` keeps the whole rest of the file.
    length: Option<f64>,
    #[serde(default = "one")]
    pitch: f64,
    #[serde(default = "default_fade")]
    fade: f64,
    #[serde(default)]
    gain: f64,
}

fn one() -> f64 {
    1.0
}

fn default_fade() -> f64 {
    0.15
}

pub fn run(module: &str, folder: Option<&str>) -> crate::Result {
    let m = crate::modules::find(module)?;
    let path = m.dir.join("assets/sounds.toml");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("{}: {e} (no sounds for {module} yet)", path.display()))?;
    let recipe: Recipe = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let content = m.content().ok_or_else(|| format!("{module} has no content part"))?;
    let assets = content.join(format!("assets/{}", m.id));
    let out = assets.join(recipe.out.clone().unwrap_or_else(|| format!("sounds/{}", m.id)));
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let src = folder.map_or_else(|| crate::root().join("sounds-src"), PathBuf::from);
    for clip in &recipe.clip {
        let number = recipe
            .sources
            .get(&clip.source)
            .ok_or_else(|| format!("{}: clip {} names no source {}", path.display(), clip.name, clip.source))?;
        make(&out, &find(&src, *number)?, clip)?;
    }
    check_sounds_json(&assets, &out)
}

/// The downloaded file whose name starts with freesound's `number`.
fn find(folder: &Path, number: u32) -> crate::Result<PathBuf> {
    let prefix = (format!("{number}__"), format!("{number}."));
    let entries = std::fs::read_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with(&prefix.0) || name.starts_with(&prefix.1)
        })
        .ok_or_else(|| format!("freesound sound {number} is missing from {}", folder.display()))
}

fn make(out: &Path, path: &Path, clip: &Clip) -> crate::Result {
    let name = &clip.name;
    let mut filters = vec![format!(
        "atrim=start={}{}",
        clip.start,
        clip.length
            .map(|l| format!(":duration={}", l * clip.pitch))
            .unwrap_or_default()
    )];
    filters.push("asetpts=PTS-STARTPTS".into());
    filters.push(format!("asetrate=44100*{},aresample=44100", clip.pitch));
    filters.push("highpass=f=60".into());
    filters.push("loudnorm=I=-16:TP=-1.5".into());
    if clip.gain != 0.0 {
        filters.push(format!("volume={}dB", clip.gain));
    }
    if let Some(length) = clip.length {
        filters.push(format!(
            "afade=t=out:st={}:d={}",
            (length - clip.fade).max(0.0),
            clip.fade
        ));
    }
    let target = out.join(format!("{name}.ogg"));
    let status = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(path)
        .args([
            "-af",
            &filters.join(","),
            "-ac",
            "1",
            "-ar",
            "44100",
            "-c:a",
            "libvorbis",
            "-q:a",
            "5",
        ])
        .arg(&target)
        .status()
        .map_err(|e| format!("ffmpeg: {e} (is it installed?)"))?;
    if !status.success() {
        return Err(format!("ffmpeg failed on {name}.ogg ({status})"));
    }
    let file = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    println!("{name}.ogg  <- {file} [{:.2}s, x{}]", clip.start, clip.pitch);
    Ok(())
}

/// Every file the mod's `sounds.json` names must exist.
fn check_sounds_json(assets: &Path, out: &Path) -> crate::Result {
    let path = assets.join("sounds.json");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let events: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    for event in events.as_object().into_iter().flat_map(|o| o.values()) {
        for entry in event["sounds"].as_array().into_iter().flatten() {
            let id = entry.as_str().or_else(|| entry["name"].as_str()).unwrap_or("");
            let name = id.rsplit('/').next().unwrap_or(id);
            if !out.join(format!("{name}.ogg")).exists() {
                return Err(format!("sounds.json names {name}.ogg, which was not made"));
            }
        }
    }
    Ok(())
}
