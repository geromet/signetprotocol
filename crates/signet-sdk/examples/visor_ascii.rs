//! Un "visor" en la terminal: el ejemplo más simple de traductor. Implementa
//! `TraductorMundo` (terreno → caracteres) y `TraductorPresentacion` (cuerpos y
//! eventos → letras y mensajes) y entra al mundo como observador.
//!
//!   cargo run --example visor_ascii -- 192.168.1.212

use std::time::Duration;

use signet_sdk::cliente::Cliente;
use signet_sdk::protocolo::{Evento, JugadorNet, OBSERVADOR, Terreno};
use signet_sdk::traductor::{TraductorMundo, TraductorPresentacion};

/// Traductor de mundo: cada celda de 1 m es un carácter según su altura.
struct MundoAscii;

impl TraductorMundo for MundoAscii {
    type Escena = Vec<Vec<char>>;

    fn es_nativo(&self, terreno: &Terreno) -> bool {
        terreno.fuente.starts_with("ascii:")
    }

    fn traducir(&mut self, t: &Terreno) -> Self::Escena {
        let rampa = ['.', ':', '-', '=', '+', '*', '#', '%', '@'];
        (0..t.lado)
            .map(|j| {
                (0..t.lado)
                    .map(|i| {
                        let k = j * t.lado + i;
                        if t.es_solido(k) {
                            ' '
                        } else {
                            rampa[((t.alturas[k] as i32 + 4).clamp(0, 8)) as usize]
                        }
                    })
                    .collect()
            })
            .collect()
    }
}

/// Traductor de presentación: bots = 'z', jugadores = inicial de su juego.
struct PresentacionAscii {
    marcas: Vec<(f32, f32, char)>,
    mensajes: Vec<String>,
}

impl TraductorPresentacion for PresentacionAscii {
    fn jugadores(&mut self, jugadores: &[JugadorNet], _mi_id: Option<u32>) {
        self.marcas = jugadores
            .iter()
            .filter(|j| j.vivo)
            .map(|j| (j.pos[0], j.pos[2], if j.juego == "bot" { 'z' } else { j.juego.chars().next().unwrap_or('?').to_ascii_uppercase() }))
            .collect();
    }

    fn evento(&mut self, e: &Evento) {
        if let Evento::Muerte { victima, autor, .. } = e {
            self.mensajes.push(format!("#{autor} killed #{victima}"));
        }
    }
}

fn main() {
    let host = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1".into());
    let c = Cliente::conectar(&host, OBSERVADOR);
    let mut mundo = MundoAscii;
    let mut pres = PresentacionAscii { marcas: Vec::new(), mensajes: Vec::new() };
    let mut escena: Option<(Vec<Vec<char>>, usize)> = None;
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(500));
        let Some(t) = c.terreno() else {
            println!("{}", c.aviso());
            continue;
        };
        let (base, lado) = escena.get_or_insert_with(|| (mundo.traducir(&t), t.lado));
        pres.jugadores(&c.jugadores(), c.mi_id());
        for e in c.tomar_eventos() {
            pres.evento(&e);
        }
        let mut lienzo = base.clone();
        let mitad = (*lado / 2) as f32;
        for &(x, z, ch) in &pres.marcas {
            let (i, j) = ((x + mitad) as usize, (z + mitad) as usize);
            if let Some(celda) = lienzo.get_mut(j).and_then(|f| f.get_mut(i)) {
                *celda = ch;
            }
        }
        // Se dibuja a la mitad de resolución horizontal para que quepa en la terminal.
        print!("\x1b[2J\x1b[H{} · {} · {} bodies\n", t.fuente, c.modo(), pres.marcas.len());
        for fila in lienzo.iter().step_by(2) {
            println!("{}", fila.iter().step_by(1).collect::<String>());
        }
        for m in pres.mensajes.iter().rev().take(3) {
            println!("{m}");
        }
    }
}
