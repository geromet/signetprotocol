//! Predicción y reconciliación del propio cuerpo, sin motor ni red.
//!
//! Funciona como en Quake III:
//!
//! 1. Cada [`TIC`] (1/20 s) se genera un comando numerado ([`Intencion::seq`]),
//!    se aplica en local con las reglas del modo y se envía al servidor.
//! 2. El servidor aplica cada comando exactamente una vez y devuelve en
//!    [`JugadorNet::seq`] el último que aplicó.
//! 3. El cliente parte de la posición que le da el servidor y vuelve a aplicar
//!    los comandos que aún no le ha confirmado. Con las mismas reglas y los
//!    mismos comandos, ambos llegan al mismo punto.
//! 4. Para que se vea suave, la posición que se dibuja se interpola entre el
//!    estado anterior y el posterior al último comando.
//!
//! [`Desincronia`] mide cuánto se equivocó la predicción en cada confirmación.

use std::collections::VecDeque;

use crate::protocolo::{Intencion, JugadorNet, Terreno};
use crate::reglas::{TIC, doom as reglas};

/// Posición en el plano (x, z) y altura de los pies.
pub type Estado = ((f32, f32), f32);

/// Cuánto se separan la predicción y el servidor.
#[derive(Clone, Debug, Default)]
pub struct Desincronia {
    /// Posición del cuerpo según el servidor (la última que llegó).
    pub servidor: Option<[f32; 3]>,
    /// Posición predicha que se dibuja.
    pub prediccion: Option<[f32; 3]>,
    /// Último comando confirmado por el servidor.
    pub confirmado: u32,
    /// Último comando generado.
    pub generado: u32,
    /// Error de la última confirmación: servidor frente a lo predicho para ese comando.
    pub ultimo_error: f32,
    pub max_error: f32,
    /// Confirmaciones en las que el servidor acabó a más de 5 cm de la predicción.
    pub correcciones: u32,
    /// Las últimas correcciones (seq, metros).
    pub historial: VecDeque<(u32, f32)>,
    /// El servidor no numera comandos (versión antigua): solo se adelanta un poco.
    pub legado: bool,
}

/// Estado de la predicción de un jugador.
#[derive(Default)]
pub struct Prediccion {
    pendientes: VecDeque<(Intencion, Estado)>,
    seq: u32,
    acumulado: f32,
    vivo: bool,
    revisado: u32,
    pub desincronia: Desincronia,
}

/// Comandos sin confirmar que se guardan como mucho (2 s a 20 Hz).
const MAX_PENDIENTES: usize = 40;

/// Aplica los primeros `cuantos` comandos pendientes desde el estado del servidor.
fn repetir(t: &Terreno, base: Estado, pendientes: &VecDeque<(Intencion, Estado)>, cuantos: usize) -> Estado {
    pendientes.iter().take(cuantos).fold(base, |(p, y), (c, _)| reglas::mover(t, p, y, c, TIC))
}

/// Orientación hacia el pasillo más largo desde un punto (para no aparecer mirando a una pared).
pub fn orientacion_despejada(t: &Terreno, p: (f32, f32), y: f32) -> f32 {
    (0..16)
        .map(|k| k as f32 * std::f32::consts::TAU / 16.0)
        .max_by(|a, b| {
            let d = |yaw: f32| reglas::hasta_pared(t, p, y + reglas::PASO_MAX, reglas::adelante(yaw), 30.0);
            d(*a).total_cmp(&d(*b))
        })
        .unwrap_or(0.0)
}

/// Resultado de un fotograma de predicción.
pub struct Fotograma {
    /// Dónde dibujar los pies del jugador (None si aún no hay cuerpo).
    pub posicion: Option<[f32; 3]>,
    /// Si el cuerpo acaba de (re)aparecer: el motor puede reorientar la cámara.
    pub reaparece: bool,
}

impl Prediccion {
    /// Último comando generado.
    pub fn seq(&self) -> u32 {
        self.seq
    }

    /// Avanza `dt` segundos: confirma lo que el servidor ya aplicó, genera los
    /// comandos de los tics vencidos (que se añaden a `salida` para enviarlos)
    /// y devuelve dónde dibujar al jugador.
    pub fn avanzar(&mut self, t: &Terreno, yo: &JugadorNet, intencion: &Intencion, dt: f32, salida: &mut Vec<Intencion>) -> Fotograma {
        let servidor: Estado = ((yo.pos[0], yo.pos[2]), yo.pos[1]);
        let reaparece = yo.vivo && !self.vivo;
        self.vivo = yo.vivo;
        let d = &mut self.desincronia;
        d.servidor = Some(yo.pos);
        d.legado = yo.seq == 0 && self.seq > MAX_PENDIENTES as u32;

        if !yo.vivo {
            self.pendientes.clear();
        }
        while let Some((c, estado)) = self.pendientes.front().copied() {
            if c.seq > yo.seq {
                break;
            }
            self.pendientes.pop_front();
            if c.seq == yo.seq && yo.seq > self.revisado {
                self.revisado = yo.seq;
                let error = ((estado.0.0 - servidor.0.0).powi(2) + (estado.0.1 - servidor.0.1).powi(2) + (estado.1 - servidor.1).powi(2)).sqrt();
                d.ultimo_error = error;
                d.max_error = d.max_error.max(error);
                if error > 0.05 {
                    d.correcciones += 1;
                    d.historial.push_front((c.seq, error));
                    d.historial.truncate(6);
                }
            }
        }
        d.confirmado = yo.seq;
        let limite = if d.legado { 3 } else { MAX_PENDIENTES };

        self.acumulado = (self.acumulado + dt).min(0.25);
        while self.acumulado >= TIC {
            self.acumulado -= TIC;
            self.seq += 1;
            let c = Intencion { seq: self.seq, ..*intencion };
            salida.push(c);
            if yo.vivo {
                let antes = repetir(t, servidor, &self.pendientes, self.pendientes.len());
                let despues = reglas::mover(t, antes.0, antes.1, &c, TIC);
                self.pendientes.push_back((c, despues));
                while self.pendientes.len() > limite {
                    self.pendientes.pop_front();
                }
            }
        }
        self.desincronia.generado = self.seq;

        let n = self.pendientes.len();
        let actual = repetir(t, servidor, &self.pendientes, n);
        let anterior = if n > 0 { repetir(t, servidor, &self.pendientes, n - 1) } else { actual };
        let k = self.acumulado / TIC;
        let p = [
            anterior.0.0 + (actual.0.0 - anterior.0.0) * k,
            anterior.1 + (actual.1 - anterior.1) * k,
            anterior.0.1 + (actual.0.1 - anterior.0.1) * k,
        ];
        self.desincronia.prediccion = Some(p);
        Fotograma { posicion: Some(p), reaparece }
    }
}
