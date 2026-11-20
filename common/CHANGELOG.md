# Changelog

Each version's section is its registry changelog (`cargo creature release`).

## 0.3.0

- Built for Sunrise's 2026-11-20 Nightly (9425651); the mods built on it need this version (`^0.3`). Each package
  lists what it uses of the game's API (`uses.json`).
- Water creatures (`creature::aquatic`): they never drown and don't float to the surface, are beached out of water
  (flopping, drying out), caught in a water bucket into their mod's bucket and let out again, kept for good, and spawn
  in groups of one look under the water of their biomes.
- Touches (`creature::touch`): what touches a creature is hurt by it at most once every so many ticks.
- Hunters (`creature::prey`): a hunter within range goes for its prey and eats it (sea turtles and jellyfish).
- `mob::player_of`; the creative tab edits its contents through the game's new edit handle.

## 0.2.0

- Built for Sunrise's 2026-11-19 Nightly (520fc48); the mods built on it need this version (`^0.2`).
- The lights of glowing creatures (fireflies, fairies, storm spirits) share the game's 8 fairly: the creatures
  nearest a player get them, instead of one mod taking every slot.
- Fighting back is the library's (`creature::anger`), shared by the scorpions and the storm spirits; with
  `give_effect`, `nearest_player` and `distance`.
- The glass jar places each creature itself (`Species::draw_scale`, `draw_lift`).

## 0.1.1

- First release on Sunrise. (0.1.0 was never published: 0.1.1 is the library's first version players get.)
