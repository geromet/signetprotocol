//! # Signet SDK (beta)
//!
//! The **Signet Protocol** lets different games join the same world: a
//! neutral, authoritative server owns the geometry, the rules and the bodies;
//! every player sees and controls it with *their own* game through a
//! **translator**.
//!
//! The SDK has six parts:
//!
//! * [`protocolo`]: the neutral Signet/1 vocabulary (terrain, intents, state
//!   and events) and its wire format (JSON lines over TCP, port 7777).
//! * [`reglas`]: shared game modes. Server and clients run the same code, so
//!   client-side prediction matches the server.
//! * [`prediccion`]: prediction and reconciliation with numbered commands,
//!   like Quake III `usercmd`s, independent of any engine.
//! * [`cliente`]: a ready-to-use connection for any engine: it receives the
//!   world, sends the player's intents and tells you where to draw the player.
//! * [`mundo`]: native Signet worlds (`signet:<id>`), with lights, objects and
//!   sky, that every other game receives translated.
//! * [`traductor`] and [`conformidad`]: the interfaces each game implements
//!   and the checks it must pass before publishing.
//!
//! The Signet/1 wire vocabulary is in Spanish (`Intencion`, `avance`,
//! `Terreno`…), and so are the Rust identifiers that mirror it; the
//! documentation explains every field in English.
//!
//! ```no_run
//! use signet_sdk::{cliente::Cliente, protocolo::Intencion};
//!
//! let mut c = Cliente::conectar("192.168.1.212", "my-game");
//! loop {
//!     c.fijar_intencion(Intencion { avance: 1.0, ..Default::default() });
//!     if let Some(pos) = c.avanzar(1.0 / 60.0) {
//!         println!("draw the camera at {pos:?}");
//!     }
//!     std::thread::sleep(std::time::Duration::from_millis(16));
//! }
//! ```

pub mod cliente;
pub mod conformidad;
pub mod mundo;
pub mod prediccion;
pub mod protocolo;
pub mod reglas;
pub mod traductor;

/// SDK version (the crate's).
pub const VERSION_SDK: &str = env!("CARGO_PKG_VERSION");
