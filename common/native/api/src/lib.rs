//! Horizon Creature, the library of the series (common/SPECS.md).
//!
//! The mods of the series reach it as the `creature` crate: the launcher adds it to their `[api]` package because
//! their `horizon-mixin.toml` lists it in `[dependencies]` (Horizon's docs/native-mods.md, "Dependencies between trait
//! mods"). What is here is shared by several mods; what only one mod needs stays in that mod.
//!
//! - `model`: draws a Blockbench model converted by `cargo creature model` (SPECS §3.1).
//! - `sounds`: a mod mob's own sounds, since Horizon prefixes mob sounds with `minecraft:` (SPECS §3.2).
pub mod model;
pub mod sounds;

use horizon_client::mod_api::Hooks;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Creature)
}

/// The library's own hooks: none yet, the mods call its functions from theirs.
struct Creature;

impl Hooks for Creature {}
