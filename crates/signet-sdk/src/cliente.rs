//! Cliente listo para cualquier motor: conecta en segundo plano, guarda lo que
//! manda el servidor y, en cada fotograma, envía las intenciones del jugador y
//! devuelve dónde dibujarlo (con predicción).
//!
//! No bloquea nunca: llama a [`Cliente::avanzar`] desde el bucle del motor.

use std::io::{BufRead, BufReader};
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::mundo::Mapa;
use crate::prediccion::{Desincronia, Prediccion};
use crate::protocolo::{self, DelCliente, DelServidor, Evento, Intencion, JugadorNet, Terreno};

/// En qué punto está la conexión.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EstadoConexion {
    Conectando,
    Conectado,
    Desconectado,
}

#[derive(Default)]
struct Bandeja {
    estado: Option<EstadoConexion>,
    aviso: String,
    tu_id: Option<u32>,
    terreno: Option<Arc<Terreno>>,
    mapa: Option<Arc<Mapa>>,
    modo: String,
    version_servidor: u32,
    jugadores: Vec<JugadorNet>,
    eventos: Vec<Evento>,
    salida: Option<TcpStream>,
    cerrar: bool,
}

/// Connection to a Signet server.
pub struct Cliente {
    bandeja: Arc<Mutex<Bandeja>>,
    intencion: Intencion,
    prediccion: Prediccion,
    yaw_aparicion: Option<f32>,
}

impl Cliente {
    /// Conecta con `host` (puerto [`protocolo::PUERTO`]) presentándose como `juego`
    /// ("doom", "minecraft", "mi-juego"…; [`protocolo::OBSERVADOR`] para solo mirar).
    /// Reintenta cada medio segundo hasta conseguirlo.
    pub fn conectar(host: &str, juego: &str) -> Self {
        let direccion = if host.contains(':') { host.to_string() } else { format!("{host}:{}", protocolo::PUERTO) };
        Self::conectar_a(direccion, juego.to_string())
    }

    fn conectar_a(direccion: String, juego: String) -> Self {
        let bandeja = Arc::new(Mutex::new(Bandeja { estado: Some(EstadoConexion::Conectando), ..Default::default() }));
        let hilo = bandeja.clone();
        std::thread::spawn(move || {
            let stream = loop {
                if hilo.lock().unwrap().cerrar {
                    return;
                }
                match TcpStream::connect(&direccion) {
                    Ok(s) => break s,
                    Err(e) => {
                        hilo.lock().unwrap().aviso = format!("connecting to {direccion}: {e}");
                        std::thread::sleep(Duration::from_millis(500));
                    }
                }
            };
            let _ = stream.set_nodelay(true);
            let Ok(mut salida) = stream.try_clone() else { return };
            let _ = protocolo::enviar(&mut salida, &DelCliente::Hola { juego, version: protocolo::VERSION_PROTOCOLO });
            {
                let mut b = hilo.lock().unwrap();
                b.salida = Some(salida);
                b.estado = Some(EstadoConexion::Conectado);
                b.aviso = format!("connected to {direccion}");
            }
            for linea in BufReader::new(stream).lines() {
                let Ok(linea) = linea else { break };
                let Ok(msg) = serde_json::from_str::<DelServidor>(&linea) else { continue };
                let mut b = hilo.lock().unwrap();
                match msg {
                    DelServidor::Bienvenida { tu_id, terreno, version, modo, mapa } => {
                        b.tu_id = Some(tu_id);
                        // Si el servidor no manda el mapa nativo pero es uno incluido, se usa el local.
                        b.mapa = mapa.or_else(|| crate::mundo::mapa_nativo(&terreno.fuente)).map(Arc::new);
                        b.terreno = Some(Arc::new(terreno));
                        b.modo = modo;
                        b.version_servidor = version;
                    }
                    DelServidor::Estado { jugadores, eventos } => {
                        b.jugadores = jugadores;
                        b.eventos.extend(eventos);
                    }
                }
            }
            let mut b = hilo.lock().unwrap();
            b.estado = Some(EstadoConexion::Desconectado);
            b.salida = None;
        });
        Self { bandeja, intencion: Intencion { arma: protocolo::ARMA_PISTOLA, ..Default::default() }, prediccion: Prediccion::default(), yaw_aparicion: None }
    }

    pub fn estado(&self) -> EstadoConexion {
        self.bandeja.lock().unwrap().estado.unwrap_or(EstadoConexion::Conectando)
    }

    /// Último aviso de la conexión, legible por una persona.
    pub fn aviso(&self) -> String {
        self.bandeja.lock().unwrap().aviso.clone()
    }

    /// El mundo neutral (llega una vez, en la bienvenida).
    pub fn terreno(&self) -> Option<Arc<Terreno>> {
        self.bandeja.lock().unwrap().terreno.clone()
    }

    /// The native map, if the world is a native Signet world (`fuente = "signet:…"`).
    pub fn mapa(&self) -> Option<Arc<Mapa>> {
        self.bandeja.lock().unwrap().mapa.clone()
    }

    /// Reglas del modo de juego del servidor ("doom:deathmatch"…).
    pub fn modo(&self) -> String {
        self.bandeja.lock().unwrap().modo.clone()
    }

    pub fn version_servidor(&self) -> u32 {
        self.bandeja.lock().unwrap().version_servidor
    }

    /// Id del propio cuerpo.
    pub fn mi_id(&self) -> Option<u32> {
        self.bandeja.lock().unwrap().tu_id
    }

    /// Todos los cuerpos (bots incluidos) en la última foto del servidor.
    pub fn jugadores(&self) -> Vec<JugadorNet> {
        self.bandeja.lock().unwrap().jugadores.clone()
    }

    /// El propio cuerpo según el servidor.
    pub fn yo(&self) -> Option<JugadorNet> {
        let b = self.bandeja.lock().unwrap();
        let id = b.tu_id?;
        b.jugadores.iter().find(|j| j.id == id).cloned()
    }

    /// Eventos llegados desde la última llamada (disparos, daño, muertes…).
    pub fn tomar_eventos(&self) -> Vec<Evento> {
        std::mem::take(&mut self.bandeja.lock().unwrap().eventos)
    }

    /// Lo que el jugador quiere hacer ahora (el traductor de entrada lo rellena).
    pub fn fijar_intencion(&mut self, i: Intencion) {
        self.intencion = i;
    }

    pub fn intencion(&self) -> Intencion {
        self.intencion
    }

    /// Si el cuerpo acaba de reaparecer, hacia dónde conviene mirar (una sola vez).
    pub fn tomar_yaw_aparicion(&mut self) -> Option<f32> {
        self.yaw_aparicion.take()
    }

    /// Avanza `dt` segundos: predice, envía los comandos de los tics vencidos y
    /// devuelve la posición de los pies donde dibujar al jugador.
    pub fn avanzar(&mut self, dt: f32) -> Option<[f32; 3]> {
        let (terreno, yo) = {
            let b = self.bandeja.lock().unwrap();
            let yo = b.tu_id.and_then(|id| b.jugadores.iter().find(|j| j.id == id).cloned());
            (b.terreno.clone(), yo)
        };
        let (Some(t), Some(yo)) = (terreno, yo) else { return None };
        let mut salida = Vec::new();
        let f = self.prediccion.avanzar(&t, &yo, &self.intencion, dt, &mut salida);
        if f.reaparece {
            let yaw = crate::prediccion::orientacion_despejada(&t, (yo.pos[0], yo.pos[2]), yo.pos[1]);
            self.intencion.yaw = yaw;
            self.yaw_aparicion = Some(yaw);
        }
        let mut b = self.bandeja.lock().unwrap();
        let fallo = match b.salida.as_mut() {
            Some(s) => salida.iter().any(|c| protocolo::enviar(s, &DelCliente::Intencion(*c)).is_err()),
            None => false,
        };
        if fallo {
            b.salida = None;
        }
        f.posicion
    }

    /// Cuánto se separan la predicción y el servidor (para depurar).
    pub fn desincronia(&self) -> &Desincronia {
        &self.prediccion.desincronia
    }
}

impl Drop for Cliente {
    fn drop(&mut self) {
        let mut b = self.bandeja.lock().unwrap();
        b.cerrar = true;
        if let Some(s) = b.salida.take() {
            let _ = s.shutdown(Shutdown::Both);
        }
    }
}
