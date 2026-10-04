//! Protocolo neutral entre servidor y clientes: JSON, un mensaje por línea,
//! sobre TCP. Solo viajan alturas, posiciones e IDs; nunca archivos de juegos.

use serde::{Deserialize, Serialize};

pub const PUERTO: u16 = 7777;

/// Valor de `Hola::juego` para conexiones que solo miran (no tienen avatar).
pub const OBSERVADOR: &str = "observador";

/// Terreno neutral: una columna de altura por celda de 1×1 m alrededor del
/// origen. La celda `(i, j)` cubre x ∈ [i - lado/2, i - lado/2 + 1) y lo mismo en z.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Terreno {
    /// Punto del mundo de Minecraft que hace de origen neutral (coordenadas de bloque).
    pub origen: [i32; 3],
    pub lado: usize,
    /// Altura de la superficie de cada celda, relativa a `origen[1]`, fila a fila (z, luego x).
    pub alturas: Vec<i16>,
    /// Si la superficie de la celda es agua.
    pub agua: Vec<bool>,
    /// Nivel del mar relativo a `origen[1]`.
    pub nivel_mar: i16,

    /// De qué juego y mapa sale el mundo: "minecraft:overworld", "doom:E1M1"…
    /// Un visor del mismo juego puede dibujarlo nativo; los demás lo traducen.
    #[serde(default)]
    pub fuente: String,
    /// Altura del techo de cada celda; `SIN_TECHO` = cielo abierto. Vacío = todo cielo.
    #[serde(default)]
    pub techo: Vec<i16>,
    /// Celdas que no se pueden pisar (paredes, vacío). Vacío = todas transitables.
    /// En una celda sólida, `alturas` es la altura de lo alto de la pared.
    #[serde(default)]
    pub solido: Vec<bool>,
    /// Material neutral de cada celda (suelo, o pared si es sólida): índice en `paleta`.
    #[serde(default)]
    pub material: Vec<u8>,
    /// Nombres neutrales de material: "core:piedra", "core:metal", "core:toxico"…
    #[serde(default)]
    pub paleta: Vec<String>,
    /// Luz ambiente de cada celda, 0–255.
    #[serde(default)]
    pub luz: Vec<u8>,
    /// Puntos de aparición en coordenadas neutrales (x, z).
    #[serde(default)]
    pub apariciones: Vec<[f32; 2]>,
    /// Celdas cuyo techo es cielo abierto (su `techo` sigue siendo la altura
    /// real, para saber hasta dónde suben las fachadas). Vacío = se usa
    /// `techo == SIN_TECHO` para saberlo, como en los servidores antiguos.
    #[serde(default)]
    pub cielo: Vec<bool>,
}

pub const SIN_TECHO: i16 = i16::MAX;

impl Terreno {
    fn indice(&self, x: f32, z: f32) -> usize {
        let mitad = (self.lado / 2) as f32;
        let i = (x + mitad).floor().clamp(0.0, (self.lado - 1) as f32) as usize;
        let j = (z + mitad).floor().clamp(0.0, (self.lado - 1) as f32) as usize;
        j * self.lado + i
    }

    pub fn es_solido(&self, k: usize) -> bool {
        self.solido.get(k).copied().unwrap_or(false)
    }

    /// Si un jugador puede estar en esa posición neutral.
    pub fn transitable(&self, x: f32, z: f32) -> bool {
        let lim = self.limite();
        x.abs() <= lim && z.abs() <= lim && !self.es_solido(self.indice(x, z))
    }

    pub fn techo_de(&self, k: usize) -> i16 {
        self.techo.get(k).copied().unwrap_or(SIN_TECHO)
    }

    /// Si sobre la celda se ve el cielo (no hay techo que dibujar).
    pub fn es_cielo(&self, k: usize) -> bool {
        match self.cielo.get(k) {
            Some(c) => *c,
            None => self.techo_de(k) == SIN_TECHO,
        }
    }

    pub fn material_de(&self, k: usize) -> &str {
        self.material.get(k).and_then(|m| self.paleta.get(*m as usize)).map_or("", |s| s.as_str())
    }

    pub fn tiene_interiores(&self) -> bool {
        !self.techo.is_empty()
    }

    pub fn celda(&self, i: usize, j: usize) -> (f32, bool) {
        let k = j * self.lado + i;
        (self.alturas[k] as f32, self.agua[k])
    }

    /// Altura del suelo en una posición neutral (la celda que la contiene).
    pub fn altura(&self, x: f32, z: f32) -> f32 {
        self.alturas[self.indice(x, z)] as f32
    }

    pub fn limite(&self) -> f32 {
        (self.lado / 2) as f32 - 1.0
    }
}

/// Versión del protocolo neutral. Sube cuando cambia algo incompatible.
pub const VERSION_PROTOCOLO: u32 = 1;

/// Armas neutrales (arquetipos). Cada traductor las dibuja a su manera.
pub const ARMA_PUNO: u8 = 0;
pub const ARMA_PISTOLA: u8 = 1;
pub const ARMA_ESCOPETA: u8 = 2;

fn verdadero() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JugadorNet {
    pub id: u32,
    /// Posición de los pies en coordenadas neutrales.
    pub pos: [f32; 3],
    /// Juego con el que entró ("minecraft", "doom", "bot"…).
    pub juego: String,
    /// Hacia dónde mira, en radianes (0 = norte, -z; positivo gira a la izquierda).
    #[serde(default)]
    pub yaw: f32,
    /// Vida 0–100 (los traductores la pasan a su escala: corazones, %…).
    #[serde(default)]
    pub vida: i32,
    #[serde(default)]
    pub armadura: i32,
    #[serde(default)]
    pub arma: u8,
    /// Munición del arma actual.
    #[serde(default)]
    pub municion: i32,
    #[serde(default)]
    pub frags: i32,
    #[serde(default = "verdadero")]
    pub vivo: bool,
    /// Último comando numerado (`Intencion::seq`) que el servidor ya aplicó a
    /// este jugador: el cliente parte de aquí y repite los que faltan.
    #[serde(default)]
    pub seq: u32,
}

/// Lo que pasa en un tick y cada visor tiene que mostrar a su manera.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Evento {
    /// Alguien disparó: desde dónde y hasta dónde llegó el disparo (pared o blanco).
    Disparo { de: u32, arma: u8, desde: [f32; 3], hasta: [f32; 3] },
    /// Un jugador recibió daño, y desde dónde le llegó (para el indicador de dirección).
    Danio { a: u32, de: u32, cantidad: i32, desde: [f32; 3] },
    Muerte { victima: u32, autor: u32, arma: u8 },
    Reaparicion { id: u32 },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DelServidor {
    Bienvenida {
        tu_id: u32,
        terreno: Terreno,
        #[serde(default)]
        version: u32,
        /// Reglas del modo de juego ("doom:deathmatch"…).
        #[serde(default)]
        modo: String,
        /// For native Signet worlds (`fuente = "signet:…"`), the full map:
        /// lights, objects and sky for Signet viewers.
        /// Los demás traductores usan `terreno` y pueden ignorarlo.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mapa: Option<crate::mundo::Mapa>,
    },
    Estado {
        jugadores: Vec<JugadorNet>,
        #[serde(default)]
        eventos: Vec<Evento>,
    },
}

/// Lo que el jugador quiere hacer en este momento. El servidor decide qué pasa.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Intencion {
    /// -1 (atrás) … 1 (adelante).
    pub avance: f32,
    /// -1 (izquierda) … 1 (derecha).
    pub lateral: f32,
    pub yaw: f32,
    pub correr: bool,
    pub disparar: bool,
    pub usar: bool,
    pub arma: u8,
    /// Número de comando (1, 2, 3…), uno por tic de servidor como los usercmd
    /// de Quake III: el servidor aplica cada uno exactamente una vez. 0 = sin
    /// numerar (adaptadores antiguos): se usa la última intención en cada tic.
    #[serde(default)]
    pub seq: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DelCliente {
    Hola {
        juego: String,
        #[serde(default)]
        version: u32,
    },
    /// Dónde quiere estar el jugador (el servidor decide la altura). Para
    /// adaptadores que solo conocen la posición, como la pasarela de Minecraft.
    Posicion { x: f32, z: f32 },
    Intencion(Intencion),
}

pub fn enviar<T: Serialize>(stream: &mut std::net::TcpStream, msg: &T) -> std::io::Result<()> {
    use std::io::Write;
    let mut linea = serde_json::to_vec(msg).map_err(std::io::Error::other)?;
    linea.push(b'\n');
    stream.write_all(&linea)
}
