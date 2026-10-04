# Changelog

All notable changes are listed here. The SDK follows [Semantic Versioning](https://semver.org/) once it leaves beta. The wire protocol is versioned separately: **Signet/1** only grows with optional fields.

## 0.1.0-beta.1 · 2026-10-04

First public release.

- **Protocol:** Signet/1 (JSON lines over TCP, port 7777): welcome, state, events and numbered intents.
- **Rust SDK (`signet-sdk`):** protocol types, game rules, client-side prediction with reconciliation, an engine-agnostic client, native worlds (`.mvm`), translator traits and the conformance suite (T01–T07, R01–R02, M01–M04).
- **C interface (`signet-ffi`)** and **C# binding** (preview, not yet tested in Unity).
- **TypeScript / Node binding (`@signet/sdk`)** (preview, no prediction).
- **Dedicated server (Docker):** `signet:nexo` (native), `doom:E1M1` (Doom 1.9 shareware) and `openarena:<map>` (OpenArena 0.8.8).
- **Documentation** site (Next.js + Nextra).

Known gaps: no jumping in `doom:deathmatch`, 2.5D terrain only, OpenArena shader textures, the Minecraft gateway still sends unnumbered intents.
