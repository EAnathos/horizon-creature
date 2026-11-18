//! The vulture's sounds (SPECS §7), played by the library (`creature::sounds`, common/SPECS.md §3.2):
//! `vulture:entity.vulture.*` from `assets/vulture/sounds.json`; its hurt and death are its entity type's.

pub const AMBIENT: &str = "vulture:entity.vulture.ambient";
pub const FLAP: &str = "vulture:entity.vulture.flap";
pub const TAKEOFF: &str = "vulture:entity.vulture.takeoff";
pub const EAT: &str = "vulture:entity.vulture.eat";

pub use creature::sounds::queue;
