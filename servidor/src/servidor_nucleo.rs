//! Núcleo del servidor, sin Bevy ni gráficos: red, bots y simulación del modo
//! de juego (`reglas_doom`). Lo usan la ventana del servidor (`servidor.rs`) y
//! el servidor dedicado (`src/bin/servidor_dedicado.rs`, el que corre en Docker).
//!
//! El servidor manda: los clientes envían intenciones (moverse, mirar,
//! disparar) y el servidor decide posiciones, impactos, daño y muertes, y los
//! reparte como estado + eventos.

use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use crate::protocolo::{
    self, ARMA_ESCOPETA, ARMA_PISTOLA, DelCliente, DelServidor, Evento, Intencion, JugadorNet, Terreno,
};
use crate::reglas_doom as reglas;

const BOTS: u32 = 8;
pub const TICK: f32 = 1.0 / 20.0;
const REAPARECER_JUGADOR: f32 = 3.0;
const REAPARECER_BOT: f32 = 10.0;
/// A qué distancia ve un zombi a un jugador.
const VISTA_BOT: f32 = 25.0;

/// Estado de juego de cualquier cuerpo (jugador o bot).
#[derive(Clone)]
struct Cuerpo {
    pos: (f32, f32),
    y: f32,
    yaw: f32,
    vida: i32,
    armadura: i32,
    arma: u8,
    balas: i32,
    cartuchos: i32,
    frags: i32,
    enfriamiento: f32,
    /// Segundos que faltan para reaparecer (None = vivo).
    muerto: Option<f32>,
}

impl Cuerpo {
    fn nuevo(t: &Terreno, pos: (f32, f32), vida: i32) -> Self {
        Self {
            pos,
            y: t.altura(pos.0, pos.1),
            yaw: 0.0,
            vida,
            armadura: 0,
            arma: ARMA_PISTOLA,
            balas: reglas::BALAS_INICIALES,
            cartuchos: reglas::CARTUCHOS_INICIALES,
            frags: 0,
            enfriamiento: 0.0,
            muerto: None,
        }
    }

    fn municion(&self) -> i32 {
        match self.arma {
            ARMA_PISTOLA => self.balas,
            ARMA_ESCOPETA => self.cartuchos,
            _ => 0,
        }
    }

    fn red(&self, id: u32, juego: &str) -> JugadorNet {
        JugadorNet {
            id,
            pos: [self.pos.0, self.y, self.pos.1],
            juego: juego.into(),
            yaw: self.yaw,
            vida: self.vida.max(0),
            armadura: self.armadura,
            arma: self.arma,
            municion: self.municion(),
            frags: self.frags,
            vivo: self.muerto.is_none(),
            seq: 0,
        }
    }
}

/// Comandos numerados que el servidor aplica como mucho en un tic: si llegan
/// varios juntos (la red los agrupa) se recupera, pero no se acelera a nadie.
const COMANDOS_POR_TIC: usize = 3;
/// Cola máxima: lo que sobre (un cliente que manda de más) se descarta.
const COLA_MAX: usize = 20;

struct Cliente {
    id: u32,
    juego: String,
    /// Último destino pedido por un adaptador que solo conoce posiciones.
    deseo: Option<(f32, f32)>,
    intencion: Intencion,
    /// Comandos numerados pendientes de aplicar, en orden.
    cola: std::collections::VecDeque<Intencion>,
    /// Último comando numerado aplicado (se devuelve en `JugadorNet::seq`).
    seq: u32,
    cuerpo: Option<Cuerpo>,
    salida: TcpStream,
    vivo: bool,
}

/// Zombi que pasea entre puntos al azar y dispara a quien ve.
struct Bot {
    cuerpo: Cuerpo,
    objetivo: (f32, f32),
}

/// Generador pseudoaleatorio mínimo (xorshift).
struct Azar(u64);

impl Azar {
    fn siguiente(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }

    fn entero(&mut self, max: i32) -> i32 {
        1 + (self.siguiente() * max as f32) as i32 % max
    }

    /// Un punto transitable al azar (o la primera aparición si no encuentra ninguno).
    fn punto(&mut self, t: &Terreno) -> (f32, f32) {
        let lim = t.limite();
        for _ in 0..200 {
            let (x, z) = ((self.siguiente() * 2.0 - 1.0) * lim, (self.siguiente() * 2.0 - 1.0) * lim);
            if reglas::cabe(t, x, z, t.altura(x, z)) {
                return (x, z);
            }
        }
        t.apariciones.first().map_or((0.0, 0.0), |a| (a[0], a[1]))
    }

    /// Punto de aparición de deathmatch al azar.
    fn aparicion(&mut self, t: &Terreno) -> (f32, f32) {
        if t.apariciones.is_empty() {
            return self.punto(t);
        }
        let i = (self.siguiente() * t.apariciones.len() as f32) as usize % t.apariciones.len();
        (t.apariciones[i][0], t.apariciones[i][1])
    }
}

#[derive(Default)]
struct Compartido {
    estado: String,
    terreno: Option<Terreno>,
    clientes: Vec<Cliente>,
    siguiente_id: u32,
}

/// Lo que el servidor sabe tras un tick (cada binario usa una parte).
#[allow(dead_code)]
pub struct Instantanea {
    pub estado: String,
    pub terreno: Option<Terreno>,
    pub jugadores: Vec<JugadorNet>,
    pub eventos: Vec<Evento>,
    pub clientes: usize,
}

pub struct Servidor {
    red: Arc<Mutex<Compartido>>,
    bots: Vec<Bot>,
    azar: Azar,
}

/// Disparo pendiente de resolver en este tick.
struct Tiro {
    de: u32,
    desde: (f32, f32),
    y: f32,
    yaw: f32,
    arma: reglas::Arma,
    id_arma: u8,
}

impl Servidor {
    /// Carga el mundo en otro hilo y, cuando está, escucha en el puerto 7777.
    pub fn iniciar<F>(cargar: F) -> Self
    where
        F: FnOnce(&dyn Fn(String)) -> Result<Terreno, String> + Send + 'static,
    {
        let red = Arc::new(Mutex::new(Compartido { siguiente_id: 100, ..Default::default() }));
        let hilo = red.clone();
        std::thread::spawn(move || {
            let aviso = |t: String| hilo.lock().unwrap().estado = t;
            let terreno = match cargar(&aviso) {
                Ok(t) => t,
                Err(e) => return aviso(format!("Error loading the world: {e}")),
            };
            let descripcion = format!("{} · {} ({}x{} m)", terreno.fuente, reglas::MODO, terreno.lado, terreno.lado);
            let oyente = match TcpListener::bind(("0.0.0.0", protocolo::PUERTO)) {
                Ok(o) => o,
                Err(e) => return aviso(format!("Could not open port {}: {e}", protocolo::PUERTO)),
            };
            {
                let mut c = hilo.lock().unwrap();
                c.terreno = Some(terreno);
                c.estado = format!("Listening on port {}: {descripcion}", protocolo::PUERTO);
            }
            for entrada in oyente.incoming().flatten() {
                let red = hilo.clone();
                std::thread::spawn(move || atender(red, entrada));
            }
        });
        Self { red, bots: Vec::new(), azar: Azar(0x9E37_79B9_7F4A_7C15) }
    }

    /// Un paso de simulación (llamar cada `TICK` segundos) y envío del estado a todos.
    pub fn tick(&mut self) -> Instantanea {
        let mut c = self.red.lock().unwrap();
        let Some(t) = c.terreno.clone() else {
            return Instantanea { estado: c.estado.clone(), terreno: None, jugadores: Vec::new(), eventos: Vec::new(), clientes: 0 };
        };
        let mut eventos = Vec::new();
        let mut tiros = Vec::new();

        if self.bots.is_empty() {
            for _ in 0..BOTS {
                let pos = self.azar.punto(&t);
                let objetivo = self.azar.punto(&t);
                self.bots.push(Bot { cuerpo: Cuerpo::nuevo(&t, pos, reglas::VIDA_BOT), objetivo });
            }
        }

        // Jugadores: aparecer, reaparecer y aplicar su intención.
        c.clientes.retain(|cli| cli.vivo);
        let mut humanos = 0;
        for cli in c.clientes.iter_mut().filter(|cli| cli.juego != protocolo::OBSERVADOR) {
            humanos += 1;
            let cuerpo = cli.cuerpo.get_or_insert_with(|| {
                let p = self.azar.aparicion(&t);
                Cuerpo::nuevo(&t, p, reglas::VIDA_INICIAL)
            });
            if let Some(resta) = cuerpo.muerto.as_mut() {
                // Muerto: los comandos se confirman sin mover, para no aplicarlos al reaparecer.
                if let Some(ultimo) = cli.cola.drain(..).last() {
                    cli.seq = ultimo.seq;
                }
                *resta -= TICK;
                if *resta <= 0.0 {
                    let p = self.azar.aparicion(&t);
                    let frags = cuerpo.frags;
                    *cuerpo = Cuerpo::nuevo(&t, p, reglas::VIDA_INICIAL);
                    cuerpo.frags = frags;
                    eventos.push(Evento::Reaparicion { id: cli.id });
                }
                continue;
            }
            cuerpo.enfriamiento = (cuerpo.enfriamiento - TICK).max(0.0);
            if let Some((x, z)) = cli.deseo.take() {
                // Adaptador por posiciones: se acepta si cabe (sin colisión fina).
                if t.transitable(x, z) {
                    cuerpo.pos = (x, z);
                    cuerpo.y = t.altura(x, z);
                }
            } else {
                let i = cli.intencion;
                if i.seq == 0 {
                    // Adaptador sin numerar: la última intención mueve un tic.
                    let (p, y) = reglas::mover(&t, cuerpo.pos, cuerpo.y, &i, TICK);
                    cuerpo.pos = p;
                    cuerpo.y = y;
                    cuerpo.yaw = i.yaw;
                } else {
                    // Cada comando numerado mueve exactamente un tic, como en el cliente:
                    // así su predicción y el servidor llegan al mismo punto.
                    for _ in 0..COMANDOS_POR_TIC {
                        let Some(c) = cli.cola.pop_front() else { break };
                        let (p, y) = reglas::mover(&t, cuerpo.pos, cuerpo.y, &c, TICK);
                        cuerpo.pos = p;
                        cuerpo.y = y;
                        cuerpo.yaw = c.yaw;
                        cli.seq = c.seq;
                    }
                }
                if reglas::es_arma_valida(i.arma) {
                    cuerpo.arma = i.arma;
                }
                if i.disparar && cuerpo.enfriamiento <= 0.0 {
                    let tiene = match cuerpo.arma {
                        ARMA_PISTOLA => cuerpo.balas > 0,
                        ARMA_ESCOPETA => cuerpo.cartuchos > 0,
                        _ => true,
                    };
                    if tiene {
                        match cuerpo.arma {
                            ARMA_PISTOLA => cuerpo.balas -= 1,
                            ARMA_ESCOPETA => cuerpo.cartuchos -= 1,
                            _ => {}
                        }
                        let arma = reglas::arma(cuerpo.arma);
                        cuerpo.enfriamiento = arma.cadencia;
                        tiros.push(Tiro { de: cli.id, desde: cuerpo.pos, y: cuerpo.y, yaw: cuerpo.yaw, arma, id_arma: cuerpo.arma });
                    }
                }
            }
        }

        // Bots: pasear, y si ven a un jugador vivo cerca, girarse y disparar.
        let blancos_humanos: Vec<(u32, (f32, f32), f32)> = c
            .clientes
            .iter()
            .filter_map(|cli| cli.cuerpo.as_ref().filter(|b| b.muerto.is_none()).map(|b| (cli.id, b.pos, b.y)))
            .collect();
        let paso = 3.0 * TICK;
        for (i, bot) in self.bots.iter_mut().enumerate() {
            let id = i as u32 + 1;
            let cu = &mut bot.cuerpo;
            if let Some(resta) = cu.muerto.as_mut() {
                *resta -= TICK;
                if *resta <= 0.0 {
                    let p = self.azar.punto(&t);
                    *cu = Cuerpo::nuevo(&t, p, reglas::VIDA_BOT);
                    eventos.push(Evento::Reaparicion { id });
                }
                continue;
            }
            cu.enfriamiento = (cu.enfriamiento - TICK).max(0.0);
            let visto = blancos_humanos
                .iter()
                .filter_map(|&(hid, hp, _)| {
                    let (dx, dz) = (hp.0 - cu.pos.0, hp.1 - cu.pos.1);
                    let d = (dx * dx + dz * dz).sqrt();
                    let dir = (dx / d.max(0.01), dz / d.max(0.01));
                    let libre = reglas::hasta_pared(&t, cu.pos, cu.y + reglas::ALTURA_OJOS, dir, d) >= d - 0.2;
                    (d < VISTA_BOT && libre).then_some((hid, d, dir))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((_, _, dir)) = visto {
                cu.yaw = (-dir.0).atan2(-dir.1);
                if cu.enfriamiento <= 0.0 {
                    let arma = reglas::arma_zombi();
                    cu.enfriamiento = arma.cadencia * (0.8 + self.azar.siguiente() * 0.6);
                    tiros.push(Tiro { de: id, desde: cu.pos, y: cu.y, yaw: cu.yaw, arma, id_arma: ARMA_PISTOLA });
                }
                continue;
            }
            let (dx, dz) = (bot.objetivo.0 - cu.pos.0, bot.objetivo.1 - cu.pos.1);
            let d = (dx * dx + dz * dz).sqrt();
            let siguiente = if d > paso { (cu.pos.0 + dx / d * paso, cu.pos.1 + dz / d * paso) } else { bot.objetivo };
            if d <= paso || !reglas::cabe(&t, siguiente.0, siguiente.1, cu.y) {
                bot.objetivo = self.azar.punto(&t);
            } else {
                cu.yaw = (-dx).atan2(-dz);
                cu.pos = siguiente;
                cu.y = t.altura(siguiente.0, siguiente.1);
            }
        }

        // Resolver disparos contra todos los cuerpos vivos (bots y jugadores).
        for tiro in tiros {
            let ojos = tiro.y + reglas::ALTURA_OJOS;
            for _ in 0..tiro.arma.perdigones {
                let desvio = (self.azar.siguiente() * 2.0 - 1.0) * tiro.arma.dispersion;
                let dir = reglas::adelante(tiro.yaw + desvio);
                let pared = reglas::hasta_pared(&t, tiro.desde, ojos, dir, tiro.arma.alcance);
                let mut mejor: Option<(u32, f32)> = None;
                let candidatos = self
                    .bots
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| b.cuerpo.muerto.is_none())
                    .map(|(i, b)| (i as u32 + 1, b.cuerpo.pos))
                    .chain(c.clientes.iter().filter_map(|cli| cli.cuerpo.as_ref().filter(|b| b.muerto.is_none()).map(|b| (cli.id, b.pos))));
                for (id, pos) in candidatos {
                    if id == tiro.de {
                        continue;
                    }
                    if let Some(d) = reglas::toca(tiro.desde, dir, pos, pared)
                        && mejor.is_none_or(|(_, m)| d < m)
                    {
                        mejor = Some((id, d));
                    }
                }
                let dist = mejor.map_or(pared, |(_, d)| d);
                let hasta = [tiro.desde.0 + dir.0 * dist, ojos, tiro.desde.1 + dir.1 * dist];
                eventos.push(Evento::Disparo { de: tiro.de, arma: tiro.id_arma, desde: [tiro.desde.0, ojos, tiro.desde.1], hasta });
                let Some((victima, _)) = mejor else { continue };
                let dano = tiro.arma.base * self.azar.entero(tiro.arma.dados);
                let desde = [tiro.desde.0, tiro.y, tiro.desde.1];
                let cuerpo = if victima <= BOTS {
                    Some(&mut self.bots[victima as usize - 1].cuerpo)
                } else {
                    c.clientes.iter_mut().find(|cli| cli.id == victima).and_then(|cli| cli.cuerpo.as_mut())
                };
                let Some(cuerpo) = cuerpo.filter(|b| b.muerto.is_none()) else { continue };
                cuerpo.vida -= dano;
                eventos.push(Evento::Danio { a: victima, de: tiro.de, cantidad: dano, desde });
                if cuerpo.vida <= 0 {
                    cuerpo.muerto = Some(if victima <= BOTS { REAPARECER_BOT } else { REAPARECER_JUGADOR });
                    eventos.push(Evento::Muerte { victima, autor: tiro.de, arma: tiro.id_arma });
                    if let Some(autor) = c.clientes.iter_mut().find(|cli| cli.id == tiro.de).and_then(|cli| cli.cuerpo.as_mut()) {
                        autor.frags += 1;
                    }
                }
            }
        }

        let mut jugadores: Vec<JugadorNet> = self.bots.iter().enumerate().map(|(i, b)| b.cuerpo.red(i as u32 + 1, "bot")).collect();
        for cli in c.clientes.iter().filter(|cli| cli.juego != protocolo::OBSERVADOR) {
            if let Some(cu) = &cli.cuerpo {
                jugadores.push(JugadorNet { seq: cli.seq, ..cu.red(cli.id, &cli.juego) });
            }
        }
        let msg = DelServidor::Estado { jugadores: jugadores.clone(), eventos: eventos.clone() };
        for cli in c.clientes.iter_mut() {
            if protocolo::enviar(&mut cli.salida, &msg).is_err() {
                cli.vivo = false;
            }
        }
        Instantanea { estado: c.estado.clone(), terreno: Some(t), jugadores, eventos, clientes: humanos }
    }
}

fn atender(red: Arc<Mutex<Compartido>>, stream: TcpStream) {
    let _ = stream.set_nodelay(true);
    let quien = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    let Ok(mut salida) = stream.try_clone() else { return };
    let id = {
        let mut c = red.lock().unwrap();
        let id = c.siguiente_id;
        c.siguiente_id += 1;
        let terreno = c.terreno.clone().expect("terreno listo antes de escuchar");
        // Los mundos nativos de Multiverso viajan completos (son contenido libre).
        let mapa = crate::mundo::mapa_nativo(&terreno.fuente);
        let bienvenida = DelServidor::Bienvenida { tu_id: id, terreno, version: protocolo::VERSION_PROTOCOLO, modo: reglas::MODO.into(), mapa };
        if protocolo::enviar(&mut salida, &bienvenida).is_err() {
            return;
        }
        c.clientes.push(Cliente {
            id,
            juego: "?".into(),
            deseo: None,
            intencion: Intencion { arma: ARMA_PISTOLA, ..Default::default() },
            cola: Default::default(),
            seq: 0,
            cuerpo: None,
            salida,
            vivo: true,
        });
        id
    };
    eprintln!("client #{id} connected from {quien}");
    for linea in BufReader::new(stream).lines() {
        let Ok(linea) = linea else { break };
        let Ok(msg) = serde_json::from_str::<DelCliente>(&linea) else { continue };
        let mut c = red.lock().unwrap();
        let Some(cli) = c.clientes.iter_mut().find(|x| x.id == id) else { break };
        match msg {
            DelCliente::Hola { juego, version } => {
                if version != protocolo::VERSION_PROTOCOLO {
                    eprintln!("client #{id}: protocol {version}, the server speaks {}", protocolo::VERSION_PROTOCOLO);
                }
                eprintln!("client #{id} joins with {juego}");
                cli.juego = juego;
            }
            DelCliente::Posicion { x, z } => cli.deseo = Some((x, z)),
            DelCliente::Intencion(i) => {
                // Los numerados se encolan en orden (sin repetidos ni atrasados).
                if i.seq > cli.seq && cli.cola.back().is_none_or(|u| i.seq > u.seq) {
                    cli.cola.push_back(i);
                    if cli.cola.len() > COLA_MAX {
                        cli.cola.pop_front();
                    }
                }
                cli.intencion = i;
            }
        }
    }
    eprintln!("client #{id} disconnected");
    if let Some(cli) = red.lock().unwrap().clientes.iter_mut().find(|x| x.id == id) {
        cli.vivo = false;
    }
}

