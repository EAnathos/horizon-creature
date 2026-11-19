# Changelog

Each version's section is its registry changelog (`cargo creature release`).

## 0.2.0

- Built for Sunrise's 2026-11-19 Nightly (520fc48); the mods built on it need this version (`^0.2`).
- The lights of glowing creatures (fireflies, fairies, storm spirits) share the game's 8 fairly: the creatures
  nearest a player get them, instead of one mod taking every slot.
- Fighting back is the library's (`creature::anger`), shared by the scorpions and the storm spirits; with
  `give_effect`, `nearest_player` and `distance`.
- The glass jar places each creature itself (`Species::draw_scale`, `draw_lift`).

## 0.1.1

- First release on Sunrise. (0.1.0 was never published: 0.1.1 is the library's first version players get.)
