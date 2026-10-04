//! Modos de juego compartidos por servidor y clientes.
//!
//! Un modo define cómo se mueven los cuerpos, qué armas hay y cómo se resuelven
//! los disparos. El servidor lo ejecuta con autoridad y cada cliente lo ejecuta
//! para predecir su propio movimiento: por eso vive en el SDK y no en un juego.
//!
//! En la beta hay un modo, [`doom`] (`"doom:deathmatch"`), con los valores del
//! código fuente de Doom publicado por id Software bajo GPL.

pub mod doom;

/// Duración de un tic de servidor: la simulación avanza en pasos fijos de 1/20 s
/// y cada comando numerado del cliente equivale a exactamente un tic.
pub const TIC: f32 = 1.0 / 20.0;
