//! Interfaces de un traductor: lo que cada juego implementa para entrar en Signet.
//!
//! Un traductor tiene hasta cuatro partes. Ninguna es obligatoria: un juego
//! puede aportar solo mapas (importador), solo un visor (mundo + presentación)
//! o el paquete completo.
//!
//! | Parte | Dirección | Qué hace |
//! |---|---|---|
//! | [`Importador`] | juego → neutral | convierte un mapa del juego en [`Terreno`] para que un servidor lo hospede |
//! | [`TraductorMundo`] | neutral → juego | construye la escena del juego a partir del [`Terreno`] |
//! | [`TraductorEntrada`] | juego → neutral | convierte teclado, mando o el estado del juego en [`Intencion`] |
//! | [`TraductorPresentacion`] | neutral → juego | muestra cuerpos y eventos con modelos, sonidos y HUD del juego |
//!
//! Los traductores leen los archivos del juego **del propio jugador**: el SDK y
//! los servidores nunca distribuyen contenido de ningún juego.

use serde::{Deserialize, Serialize};

use crate::protocolo::{Evento, Intencion, JugadorNet, Terreno};

/// Convierte mapas de un juego al terreno neutral.
pub trait Importador {
    /// Prefijo del juego en [`Terreno::fuente`]: "doom", "openarena"…
    fn juego(&self) -> &str;
    /// Mapas disponibles en la instalación del jugador o del servidor.
    fn mapas(&self) -> Vec<String>;
    /// Importa un mapa. `Terreno::fuente` debe quedar como `"<juego>:<mapa>"`.
    fn importar(&self, mapa: &str) -> Result<Terreno, String>;
}

/// Construye la escena del juego a partir del terreno neutral.
pub trait TraductorMundo {
    /// Lo que el motor necesita para dibujar (mallas, bloques, sectores…).
    type Escena;
    /// Si el terreno salió de este mismo juego (`fuente` con su prefijo), el
    /// visor puede cargar el mapa original en vez de traducirlo.
    fn es_nativo(&self, terreno: &Terreno) -> bool;
    fn traducir(&mut self, terreno: &Terreno) -> Self::Escena;
}

/// Convierte la entrada del jugador en intención neutral, en cada fotograma.
pub trait TraductorEntrada {
    fn leer(&mut self, dt: f32) -> Intencion;
}

/// Muestra en el juego lo que pasa en el mundo neutral.
pub trait TraductorPresentacion {
    /// Cuerpos de la última foto del servidor (el propio, `mi_id`, suele no dibujarse).
    fn jugadores(&mut self, jugadores: &[JugadorNet], mi_id: Option<u32>);
    /// Un evento: disparo, daño, muerte, reaparición.
    fn evento(&mut self, evento: &Evento);
}

/// Cómo se conecta el traductor con el juego.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModoIntegracion {
    /// Un programa aparte habla con el juego por su propio protocolo o consola
    /// (ej.: la pasarela de Minecraft usa el servidor oficial y RCON).
    Pasarela,
    /// El motor es de código abierto y se compila con el SDK (ej.: ioquake3).
    MotorAbierto,
    /// Mod oficial o permitido por el juego (Lua, C#, plugins).
    Mod,
    /// Un visor propio lee los archivos del jugador y dibuja como el juego
    /// (ej.: los visores Doom y OpenArena de referencia).
    Reimplementacion,
}

/// Qué partes aporta un traductor.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Capacidades {
    pub importador: bool,
    pub mundo: bool,
    pub entrada: bool,
    pub presentacion: bool,
}

/// Ficha que publica cada traductor (`signet.json`): la leen lanzadores y
/// directorios para saber qué juegos puede usar cada jugador.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Manifiesto {
    /// Identificador del juego (prefijo de `fuente`).
    pub juego: String,
    /// Nombre para mostrar.
    pub nombre: String,
    /// Versión del traductor (semver).
    pub version: String,
    /// Versión del protocolo MV que habla.
    pub protocolo: u32,
    pub integracion: ModoIntegracion,
    pub capacidades: Capacidades,
    /// Modos de juego que sabe presentar ("doom:deathmatch"…).
    #[serde(default)]
    pub modos: Vec<String>,
    /// Qué archivos del juego necesita del jugador (para explicarlo en el lanzador).
    #[serde(default)]
    pub requiere: Vec<String>,
    /// Licencia del traductor (no del juego).
    pub licencia: String,
}
