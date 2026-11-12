# Cyclops

A mod of the [Horizon Creature](../../README.md) series: a one-eyed giant of the mountains that hurls boulders, and
drops its eye. With Wyverns, the cyclops eye takes the eye of ender's place in the Wyvern Staff.

**Requires** Horizon Creature (`creature`), the series' library.

> **Status:** in progress ([SPECS.md](SPECS.md) §0). A blue one-eyed giant shepherd that appears rarely at cave mouths
> with a flock of sheep that follow it. Hostile: come near, hurt it or one of its sheep, and it brings its club down on you. Drops
> its eye (for a shepherd's crook that herds farm animals) and sometimes its club. Spawn egg, bestiary page.

- `content/`: the entity type, the spawn egg, the cyclops eye, its loot, English and French names.
- `native/`: its goals (the club's blow, defending the flock), the flock following it, its spawning at cave mouths, its
  drawing (`model.rs` from `assets/models/cyclops.bbmodel`, see [docs/MODELS.md](../../docs/MODELS.md#cyclops)).
