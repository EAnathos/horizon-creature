//! Colours and patterns (SPECS §5): one species, many looks, as vanilla's tropical fish.
//!
//! A mod describes its creature's looks in a `Scheme`: the colours, the patterns (each a texture laid over the base
//! one), and per biome how likely each is (`Variety`). The first time the library sees one of its creatures
//! (`tick`, from the mod's `entity_tick_post`), it rolls a `Look` from the variety of the biome it is in: a base
//! colour, a pattern and a pattern colour. The look is saved on the entity (`creature:look`) and kept for the drawing
//! (`look`), which draws the model twice (`draw`): the base texture tinted by the base colour, then the pattern texture
//! tinted by the pattern colour, a little grown so it shows over the base.
//!
//! The drawing reads the looks from a static map filled on the server's side: single player only, as the vulture's
//! render state (Horizon can't read a mob's attachments while drawing it; workaround for arismoko/horizon#2740, fixed
//! on the fork only).
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use horizon_client::mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, BlockEntityDraw, Compound, Holder, Host, Tag,
};

use crate::model::{self, Part};

/// A colour a creature can take: its id (saved) and its tint, 0xRRGGBB.
pub struct Colour {
    pub id: &'static str,
    pub rgb: u32,
}

/// A pattern: its id (saved) and the texture laid over the base one (`<mod>:entity/...`), none for a plain creature.
pub struct Pattern {
    pub id: &'static str,
    pub texture: Option<&'static str>,
}

/// How likely each look is in some biomes: weights by index into the scheme's colours and patterns.
pub struct Variety {
    /// The biomes it applies to (`minecraft:plains`); empty for the variety used everywhere else.
    pub biomes: &'static [&'static str],
    pub base: &'static [(usize, u32)],
    pub patterns: &'static [(usize, u32)],
    pub pattern_colours: &'static [(usize, u32)],
}

/// A creature's possible looks.
pub struct Scheme {
    pub colours: &'static [Colour],
    pub patterns: &'static [Pattern],
    /// The base texture, greyscale, tinted by the base colour.
    pub base_texture: &'static str,
    pub varieties: &'static [Variety],
}

/// One creature's look: indices into its scheme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Look {
    pub base: usize,
    pub pattern: usize,
    pub pattern_colour: usize,
}

/// Saved by id rather than by index, so that a mod reordering its lists doesn't change its creatures.
#[derive(Clone, Debug, Default, PartialEq)]
struct Saved {
    base: String,
    pattern: String,
    pattern_colour: String,
}

static LOOK: AttachmentType<Saved> = AttachmentType {
    namespace: "creature",
    name: "look",
    default: Saved::default,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

fn encode(s: &Saved) -> Result<Compound, AttachmentError> {
    let mut c = Compound::new();
    for (key, value) in [
        ("Base", &s.base),
        ("Pattern", &s.pattern),
        ("PatternColour", &s.pattern_colour),
    ] {
        c.insert(key.into(), Tag::String(value.as_str().into()));
    }
    Ok(c)
}

fn decode(c: &Compound) -> Result<Saved, AttachmentError> {
    let text = |key: &str| match c.get(&key.into()) {
        Some(Tag::String(s)) => s.to_string_lossy(),
        _ => String::new(),
    };
    Ok(Saved {
        base: text("Base"),
        pattern: text("Pattern"),
        pattern_colour: text("PatternColour"),
    })
}

/// Each creature's look, for the drawing.
static LOOKS: Mutex<BTreeMap<u32, Look>> = Mutex::new(BTreeMap::new());

/// The look of creature `entity`, once `tick` has seen it.
pub fn look(entity: u32) -> Option<Look> {
    LOOKS.lock().unwrap().get(&entity).copied()
}

/// From the mod's `entity_tick_post`, for each of its creatures: its saved look the first time, or a new one rolled for
/// the biome it is in.
pub fn tick(host: &mut dyn Host, entity: u32, scheme: &Scheme) {
    if LOOKS.lock().unwrap().contains_key(&entity) {
        return;
    }
    // Not readable yet (its first tick after loading): next tick, rather than rolling over a saved look.
    let Ok(saved) = LOOK.get_existing(host, Holder::Entity(entity)) else {
        return;
    };
    let look = match saved.and_then(|s| from_saved(scheme, &s)) {
        Some(look) => look,
        None => {
            let at = host.entity(entity).and_then(|e| host.entity_position(e));
            let biome = at.and_then(|p| host.biome([p[0].floor() as i32, p[1].floor() as i32, p[2].floor() as i32]));
            let look = roll(scheme, biome.as_deref(), &mut Random::new(entity));
            let _ = LOOK.set(host, Holder::Entity(entity), to_saved(scheme, look));
            look
        }
    };
    LOOKS.lock().unwrap().insert(entity, look);
}

/// A look as a short text (`orange/eyespots/black`, its ids), for the bestiary's morphs.
pub fn key(scheme: &Scheme, look: Look) -> String {
    let colour = |i: usize| scheme.colours.get(i).map_or("", |c| c.id);
    let pattern = scheme.patterns.get(look.pattern).map_or("", |p| p.id);
    format!("{}/{pattern}/{}", colour(look.base), colour(look.pattern_colour))
}

/// The look a `key` names, if its ids are still the scheme's.
pub fn from_key(scheme: &Scheme, key: &str) -> Option<Look> {
    let mut parts = key.split('/');
    from_saved(
        scheme,
        &Saved {
            base: parts.next()?.into(),
            pattern: parts.next()?.into(),
            pattern_colour: parts.next()?.into(),
        },
    )
}

/// The look a saved compound (`saved`, a jar's) names, if its ids are still the scheme's.
pub fn from_compound(scheme: &Scheme, look: &Compound) -> Option<Look> {
    from_saved(scheme, &decode(look).ok()?)
}

/// Creature `entity`'s saved look, as stored (`creature:look`), for the jar to carry it.
pub fn saved(host: &mut dyn Host, entity: u32) -> Option<Compound> {
    let saved = LOOK.get_existing(host, Holder::Entity(entity)).ok().flatten()?;
    encode(&saved).ok()
}

/// Gives creature `entity` a look carried by the jar (`saved`); `tick` reads it on its first tick.
pub fn restore(host: &mut dyn Host, entity: u32, look: &Compound) {
    if let Ok(saved) = decode(look) {
        let _ = LOOK.set(host, Holder::Entity(entity), saved);
    }
}

/// From the mod's `living_death` (or when it is gone): forgets its look.
pub fn forget(entity: u32) {
    LOOKS.lock().unwrap().remove(&entity);
}

fn from_saved(scheme: &Scheme, s: &Saved) -> Option<Look> {
    Some(Look {
        base: scheme.colours.iter().position(|c| c.id == s.base)?,
        pattern: scheme.patterns.iter().position(|p| p.id == s.pattern)?,
        pattern_colour: scheme.colours.iter().position(|c| c.id == s.pattern_colour)?,
    })
}

fn to_saved(scheme: &Scheme, look: Look) -> Saved {
    Saved {
        base: scheme.colours[look.base].id.into(),
        pattern: scheme.patterns[look.pattern].id.into(),
        pattern_colour: scheme.colours[look.pattern_colour].id.into(),
    }
}

/// A look for `biome`, from its variety (the first one listing it, else the one listing none).
pub fn roll(scheme: &Scheme, biome: Option<&str>, random: &mut Random) -> Look {
    let variety = scheme
        .varieties
        .iter()
        .find(|v| biome.is_some_and(|b| v.biomes.contains(&b)))
        .or_else(|| scheme.varieties.iter().find(|v| v.biomes.is_empty()));
    let Some(v) = variety else {
        return Look::default();
    };
    Look {
        base: pick(v.base, random),
        pattern: pick(v.patterns, random),
        pattern_colour: pick(v.pattern_colours, random),
    }
}

fn pick(weights: &[(usize, u32)], random: &mut Random) -> usize {
    let total: u32 = weights.iter().map(|w| w.1).sum();
    if total == 0 {
        return 0;
    }
    let mut roll = (random.next() % u64::from(total)) as u32;
    for &(index, weight) in weights {
        if roll < weight {
            return index;
        }
        roll -= weight;
    }
    0
}

/// A small random source for the rolls: seeded by the entity and the time, so two creatures spawned together differ.
pub struct Random(u64);

static SALT: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

impl Random {
    pub fn new(seed: u32) -> Random {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        let salt = SALT.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
        Random(u64::from(seed) ^ time ^ salt | 1)
    }

    /// xorshift64*. Not an `Iterator`: it never ends, and every mod calls it `next`.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

/// The pattern layer's growth, in texels (vanilla's 0.008 for the tropical fish's pattern flickered on thin wings drawn small).
const PATTERN_INFLATE: f32 = 0.02;

/// Draws `part` in `look`: the base texture in its colour, then its pattern, if any, in the pattern colour. `tint` is
/// laid over both (`model::tint(mob)` for the hurt flash).
pub fn draw(
    host: &dyn Host,
    draw: &mut BlockEntityDraw,
    part: &Part,
    scheme: &Scheme,
    look: Look,
    pose: &dyn Fn(&str) -> [f32; 3],
    tint: u32,
) {
    let colour = |i: usize| scheme.colours.get(i).map_or(model::WHITE, |c| c.rgb);
    let all = |_: &str| true;
    model::draw_layer(
        host,
        draw,
        part,
        scheme.base_texture,
        pose,
        &all,
        model::multiply(colour(look.base), tint),
        0.0,
    );
    if let Some(texture) = scheme.patterns.get(look.pattern).and_then(|p| p.texture) {
        model::draw_layer(
            host,
            draw,
            part,
            texture,
            pose,
            &all,
            model::multiply(colour(look.pattern_colour), tint),
            PATTERN_INFLATE,
        );
    }
}
