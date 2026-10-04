# Signet Protocol · beta

**Every game, one world.** Signet Protocol is an open protocol and SDK that lets players with *different* games join the same world. A neutral server owns the geometry, the rules and the bodies; every player sees and controls it with **their own** game through a translator.

The goal is a **global protocol for communication between all games**: any game can join any world, and viewers, mechanics and content can be exchanged between games, so a player can bring their own game to someone else's world and the other way around.

This repository contains the SDK to write those translators, plus the clients and the server that use them.

| Part | Path | Status |
|---|---|---|
| Rust core (`signet-sdk`): Signet/1 protocol, rules, prediction, client, native worlds, translators, conformance | `crates/signet-sdk` | beta |
| C interface (`signet.h` + `signet.dll/.so/.dylib`) | `crates/signet-ffi` | beta |
| C# (Unity / .NET) on top of the C interface | `bindings/csharp` | preview |
| TypeScript / Node (`@signet/sdk`): types and a client without prediction | `bindings/typescript` | preview |
| Dedicated server (Docker) | `servidor` | beta |
| Documentation (Next.js) | `docs` | beta |

## Five minutes

```bash
cargo run --release --example cliente_minimo -- <server-ip>      # joins, walks and reports
cargo run --release --example visor_ascii   -- <server-ip>      # a viewer in your terminal
cargo run --release --example conformidad   -- --servidor <server-ip>
```

Host a server:

```bash
docker build -f servidor/Dockerfile -t signet-server .
docker run -d --name signet -p 7777:7777 -e SIGNET_WORLD=signet:nexo signet-server
```

Read the documentation locally:

```bash
cd docs && npm install && npm run dev   # http://localhost:3000
```

## Principles

1. **The server is in charge.** Clients send intents; the server decides positions, hits and deaths.
2. **Only neutral data travels.** Heights, positions, ids and events go over the wire, never a game's files.
3. **Every player uses their own copy.** Translators read the player's own game files. Neither the SDK nor the servers distribute third-party content.
4. **No cheating, no injection into online games.** Only open-source engines, officially allowed mods, gateways over servers you control, or reimplementations.

## A note on naming

The Signet/1 wire vocabulary is in Spanish (`Intencion`, `avance`, `Terreno`…), and so are the Rust identifiers that mirror it. The documentation explains every field in English, and the C, C# and TypeScript APIs are in English. An English wire vocabulary is planned for Signet/2.

## License

Apache-2.0. See [LICENSE](LICENSE). "Signet Protocol" is a trademark of its owner. Games, their trademarks and their files belong to their respective owners.
