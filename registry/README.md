# Registry entries

The published modules' entries and icons, as [sunrise-org/sunrise-mods](https://github.com/sunrise-org/sunrise-mods)
holds them (`mods/<id>.toml`, `icons/<id>.png`, the collection `collections/sunrise_creatures.toml`), and the
series' minisign public key. The entries are written by `cargo creature release` (the collection by hand, as the
registry holds it): edit the modules (`module.toml`'s `[registry]`, `README.md`, `CHANGELOG.md`), not these files.
How a version is published: [docs/PUBLISHING.md](../docs/PUBLISHING.md).
