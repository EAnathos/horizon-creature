//! Makes the vulture's sounds from freesound files, all CC0 (public domain: nothing to credit).
//!
//! The downloaded files keep the name freesound gives them (`740195__el_boss__goose-hiss.wav`): they are found by
//! their number. Output: `mods/vulture/content/assets/vulture/sounds/vulture/*.ogg`, mono 44.1 kHz, normalized volume, like vanilla
//! sounds. Needs ffmpeg.
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const GOOSE_A: u32 = 740195; // el_boss, Goose Hiss
const GOOSE_B: u32 = 844558; // TheKingOfGeeks360, White Chinese Goose; Hiss
const CALL: u32 = 188041; // AntumDeluge, Vulture (imitation)
const WINGS: u32 = 670509; // 2create, Huge wing flaps
const TAKEOFF: u32 = 389634; // _stubb, Wing Flap 1
const EAT: u32 = 234237; // ProductionNow, Eating Juicy Meat

/// One clip: a slice of a source file, resampled by `pitch` (slower and lower below 1).
struct Clip {
    start: f64,
    /// Output length in seconds; `None` keeps the whole rest of the file.
    length: Option<f64>,
    pitch: f64,
    fade: f64,
    gain_db: f64,
}

impl Clip {
    fn new(start: f64, length: f64) -> Self {
        Clip {
            start,
            length: Some(length),
            pitch: 1.0,
            fade: 0.15,
            gain_db: 0.0,
        }
    }
    fn pitch(self, pitch: f64) -> Self {
        Clip { pitch, ..self }
    }
    fn fade(self, fade: f64) -> Self {
        Clip { fade, ..self }
    }
    fn gain(self, gain_db: f64) -> Self {
        Clip { gain_db, ..self }
    }
}

pub fn run(module: &str, folder: Option<&str>) -> crate::Result {
    if module != "vulture" {
        return Err(format!("no sounds are made for {module} yet: only vulture's (tools/src/sounds.rs)"));
    }
    let root = crate::root().join("mods/vulture");
    let src = folder.map_or_else(|| crate::root().join("sounds-src"), PathBuf::from);
    let out = root.join("content/assets/vulture/sounds/vulture");
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let source = |number: u32| find(&src, number);
    let (a, b) = (source(GOOSE_A)?, source(GOOSE_B)?);
    let make = |name: &str, path: &Path, clip: Clip| make(&out, name, path, &clip);
    // Cuts picked from the file's non-silent passages (ffmpeg `silencedetect`, thresholds from -22 to -35 dB): each
    // goose's long hiss, then the short puffs after el_boss's (1.57 s and 2.79 s). A vulture hisses lower than a
    // goose: about 4 semitones (×0.79).
    make("ambient1", &a, Clip::new(0.0, 1.7).pitch(0.79))?;
    make("ambient2", &b, Clip::new(0.0, 1.95).pitch(0.79))?;
    make("ambient3", &a, Clip::new(1.5, 2.0).pitch(0.79).fade(0.3))?;
    make(
        "call",
        &source(CALL)?,
        Clip {
            length: None,
            ..Clip::new(0.0, 0.0).pitch(0.9)
        },
    )?;
    // Hurt: a short puff, pitched down less; death: the longest hiss, lower, fading out.
    make("hurt1", &a, Clip::new(1.55, 0.35).pitch(0.9).fade(0.08))?;
    make("hurt2", &a, Clip::new(2.77, 0.35).pitch(0.9).fade(0.08))?;
    make("death", &b, Clip::new(0.0, 2.4).pitch(0.65).fade(0.9))?;
    // Flaps: three clean beats of the big wings, with their whoosh.
    let wings = source(WINGS)?;
    for (i, (start, length)) in [(1.0, 0.5), (2.42, 0.7), (3.7, 0.5)]
        .into_iter()
        .enumerate()
    {
        make(
            &format!("flap{}", i + 1),
            &wings,
            Clip::new(start, length).fade(0.12).gain(-3.0),
        )?;
    }
    make("takeoff", &source(TAKEOFF)?, Clip::new(0.0, 0.93).fade(0.3))?;
    let eat = source(EAT)?;
    make("eat1", &eat, Clip::new(0.22, 0.95).fade(0.1).gain(-4.0))?;
    make("eat2", &eat, Clip::new(5.4, 0.7).fade(0.1).gain(-4.0))?;
    check_sounds_json(&root, &out)
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
        .ok_or_else(|| {
            format!(
                "freesound sound {number} is missing from {}",
                folder.display()
            )
        })
}

fn make(out: &Path, name: &str, path: &Path, clip: &Clip) -> crate::Result {
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
    if clip.gain_db != 0.0 {
        filters.push(format!("volume={}dB", clip.gain_db));
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
    println!(
        "{name}.ogg  <- {file} [{:.2}s, x{}]",
        clip.start, clip.pitch
    );
    Ok(())
}

/// Every file `sounds.json` names must exist.
fn check_sounds_json(root: &Path, out: &Path) -> crate::Result {
    let path = root.join("content/assets/vulture/sounds.json");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let events: Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    for event in events.as_object().into_iter().flat_map(|o| o.values()) {
        for entry in event["sounds"].as_array().into_iter().flatten() {
            let id = entry
                .as_str()
                .or_else(|| entry["name"].as_str())
                .unwrap_or("");
            let name = id.rsplit('/').next().unwrap_or(id);
            if !out.join(format!("{name}.ogg")).exists() {
                return Err(format!("sounds.json names {name}.ogg, which was not made"));
            }
        }
    }
    Ok(())
}
