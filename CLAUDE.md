# horizon-vulture

Mod natif Rust pour **Minecraft Horizon** (réimplémentation Rust de Minecraft Java 26.3) : un vautour apprivoisable
(poulets crus lancés), montable en vol avec endurance, 2 coffres, serres, reproduction, stats et skins variables.

- **Specs** : `docs/SPECS.md` (source de vérité, en français ; ✅ décidé, ❓ à valider).
- **Langue** : on échange en français.

## Horizon

- Sources et docs d'Horizon installées par le launcher : `~/.local/share/horizon-launcher/native/stage/`
  - `docs/mods.md` (mods de contenu, `entity_type`, menus, spawns), `docs/native-mods.md` (hooks `native_*`, goals, rendu des mobs)
  - `crates/cargo-horizon-mod` (outil `cargo horizon-mod`), `crates/horizon-client/src/game/native_mob_draw.rs`, `mods/`
- Exemple de mod natif installé : `~/.local/share/horizon-launcher/native/installed.incoming/mod_menu/` (`horizon-native.json`, `mixins/<id>/horizon-mixin.toml`).
- Toolchain Rust 1.99.1 épinglée par le launcher. Outils MCP `horizon` disponibles (instances, mods, `game_*` pour piloter et capturer le jeu).
- Ne jamais modifier les fichiers d'Horizon dans `~/.local/share/horizon-launcher` : on les lit, on code dans ce dépôt.

## Contraintes connues de l'API

- Un type d'entité vivant du mod ne peut copier que `minecraft:zombie`, `minecraft:villager` ou `minecraft:wandering_trader`
  (on prend le marchand ambulant, dont tout le comportement passe par des goals).
- Rendu : `native_mob_submit` + `native_draw_cube` (cubes, pivots, textures `vulture:entity/...`).
- Aucun hook trouvé pour monter et piloter une entité du mod : c'est le risque n°1, que la phase 0 doit lever.

## Modèles

- Blockbench 5.2.2 (Flatpak `net.blockbench.Blockbench`) + plugin MCP jasonjgardner v1.10.1, serveur `blockbench` dans `.mcp.json`
  (Blockbench doit être ouvert). Fichiers `.bbmodel` dans `assets/models/`.
