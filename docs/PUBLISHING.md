# Publishing the series

The series is published on Sunrise's registry ([playsunrise.net/mods](https://playsunrise.net/mods)), as Sunrise's
guide says ([Publishing a mod](https://playsunrise.net/docs/publish-a-mod)): each module is its own project, a
`native-mod`, whose entry lives in [sunrise-org/sunrise-mods](https://github.com/sunrise-org/sunrise-mods)
(`mods/<id>.toml`, `icons/<id>.png`) and whose files are pinned there by their SHA-256. Every change to an entry is a
pull request a person reviews. What a listed project promises is that registry's
[MOD-POLICY.md](https://github.com/sunrise-org/sunrise-mods/blob/main/MOD-POLICY.md): the right to publish everything
in it under its licence (ours: MIT, the sounds CC0), no Minecraft game files (our packages hold only our own
namespaces), nothing hidden.

## What a published module has

- **`module.toml`'s `[registry]`**: its one-line `summary` (at most 160 characters), 1 to 3 `categories` from a mod's
  list (`tools/src/release.rs`, `CATEGORIES`), up to 10 lowercase `tags`, and its `icon`: one of its models (its
  picture in `docs/models/`) or a PNG by its path (the library's and the Vanilla Bestiary's: the guide). A module
  without the table is not published (the planned mods).
- **`sunrise-mod.toml`**: its `name`, `version`, `authors`, `license`, `description`, `homepage` and `issues`, shown by
  the game's Mods screen; the package also carries the icon (`icon.png`, made by `cargo creature pack`).
- **`README.md`**: its registry page's description (50 to 10,000 characters; its title left out, its links and images
  made absolute to this repository on GitHub).
- **`CHANGELOG.md`**: a section per version (`## 0.1.0`), its changelog in the registry and its GitHub release's notes.

`cargo creature check` holds every published module to these rules (the registry's own), so a page that would be
refused fails before release.

## Releasing a version

1. **Version.** Raise `version` in the module's `sunrise-mod.toml` and `Cargo.toml` (semver: a mod's
   `[dependencies] creature = "^0.1"` follows the library's), and write its `CHANGELOG.md` section. A published file
   never changes: any change, even one byte, is a new version.
2. **Check.** `cargo creature check` and `cargo creature native-check`, and try the module in game
   (`cargo creature install`).
3. **Release.** `cargo creature release [module...]` (default: every published module): it builds each package,
   checks it as the registry does (`cargo sunrise-mod check`), names it `target/release/<id>-<version>.smod`, signs it
   if the series has a key (below), and writes its entry `registry/mods/<id>.toml` (the new version first, the old ones
   kept) and its icon `registry/icons/<id>.png`. A version already published with another file is refused. Commit
   `registry/` with the version.
4. **GitHub release.** One release of this repository per module version, tag `<id>-<version>`, the `.smod` (and its
   `.minisig`) as its asset: the command prints each `gh release create` line.
5. **Registry pull request.** In a fork of sunrise-org/sunrise-mods, copy `registry/mods/<id>.toml` and
   `registry/icons/<id>.png`, commit with a sign-off (`git commit -s`: the registry's `dco` check), and open a pull
   request whose description links each file (its GitHub release asset) with its SHA-256 (the command prints both). A
   maintainer copies the file to the registry's staging release; its checks run; a reviewer merges. The submission
   form (playsunrise.net/mods/submit) opens the same pull request once its service runs. Questions come as comments on
   the pull request.

The library goes first: every mod requires it (`[dependencies] creature`), and a player installs it with them (the
registry lists no dependencies; the game refuses a mod without its library and says why).

## Signing (recommended)

A signed project can only be updated with its key. Once:

```bash
minisign -G -p registry/minisign.pub -s ~/.minisign/sunrise-mods.key
```

`registry/minisign.pub` is committed; the secret key and its password stay private and backed up (never in the
repository). From then on `cargo creature release` signs every file: set `SUNRISE_MODS_KEY` to the secret key's path.
Every version of a project with a key must be signed, by the same key.

## Updates and takedowns

An update is a new version through the same steps: its `[[versions]]` table is added on top of the entry. A file
can't be replaced or removed; a bad one is taken down by the registry (`revoked.toml`), which turns it off in players'
launchers.
