# Contributing to Signet Protocol

Thank you! The beta mostly needs **translators**: map importers, viewers and gateways for more games and engines.

## Before opening a PR

1. `cargo test --release` passes in the whole workspace.
2. If you contribute an importer, export a map to JSON and run the conformance suite:
   `cargo run --release --example conformidad -- your_map.json` (codes T01–T07, R01–R02).
3. If you contribute a translator, include its `signet.json` manifest (see `Manifiesto`) and pass codes M01–M04.
4. Follow the repository style: comments explain the *why*. New public documentation is written in English.

## Non-negotiable rules

- **Do not upload any game files** (maps, textures, sounds, models, executables), or links to unofficial copies. Translators read the player's own installation.
- **No injection into online games and no anti-cheat evasion.** Valid integrations are gateways over servers you control, open-source engines, mods the game officially allows, or reimplementations.
- **Respect the game's license and EULA.** If it is unclear whether an integration is allowed, open it as a proposal before writing code.

## Protocol changes

Signet/1 only grows with **optional fields that have a default value**, so older clients and servers keep working. A breaking change requires Signet/2 and an approved proposal.

## License

By contributing you agree to publish your contribution under Apache-2.0.
