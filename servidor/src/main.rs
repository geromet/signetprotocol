//! Signet Protocol dedicated server: no window, no GPU, no Bevy. This is the
//! one that runs in Docker.
//!
//!   servidor_dedicado [--mundo signet:nexo | doom:E1M1 | openarena:oa_dm1]   (or SIGNET_WORLD)

/// El protocolo, las reglas y los mundos nativos salen del SDK: servidor y
/// clientes comparten el mismo código.
mod protocolo {
    pub use signet_sdk::protocolo::*;
}

mod reglas_doom {
    pub use signet_sdk::reglas::doom::*;
}

mod mundo {
    pub use signet_sdk::mundo::*;
}

#[allow(dead_code)]
mod doom;

#[allow(dead_code)]
mod openarena;

#[allow(dead_code)]
mod quake3;

mod servidor_nucleo;

use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // --mundo or the SIGNET_WORLD variable (handy in Docker).
    let elegido = args
        .iter()
        .position(|a| a == "--mundo")
        .and_then(|i| args.get(i + 1).cloned())
        .or_else(|| std::env::var("SIGNET_WORLD").ok())
        // Name used by the first betas.
        .or_else(|| std::env::var("MULTIVERSO_MUNDO").ok())
        .unwrap_or_else(|| "doom:E1M1".into());
    let (juego, mapa) = elegido.split_once(':').map_or(("doom".to_string(), "E1M1".to_string()), |(j, m)| (j.to_string(), m.to_string()));
    if !matches!(juego.as_str(), "doom" | "openarena" | "signet" | "multiverso") {
        eprintln!("Unknown world: {elegido}. Use signet:nexo, doom:E1M1 or openarena:oa_dm1.");
        std::process::exit(2);
    }

    let mut servidor = servidor_nucleo::Servidor::iniciar(move |aviso| {
        aviso(format!("Importing {juego}:{mapa}..."));
        match juego.as_str() {
            "signet" | "multiverso" => {
                let m = mundo::incluido(&mapa).ok_or(format!("there is no native world called {mapa}"))?;
                eprintln!("Native world {}: {}x{} m, {} lights, {} objects, {} spawn points", m.nombre, m.lado, m.lado, m.luces.len(), m.objetos.len(), m.apariciones.len());
                Ok(m.a_terreno())
            }
            "openarena" => quake3::cargar(&mapa),
            _ => doom::cargar(&mapa.to_uppercase()),
        }
    });
    let mut ultimo = String::new();
    let mut siguiente = Instant::now();
    loop {
        let foto = servidor.tick();
        let linea = format!("{} | {} clients connected", foto.estado, foto.clientes);
        if linea != ultimo {
            eprintln!("{linea}");
            ultimo = linea;
        }
        siguiente += Duration::from_secs_f32(servidor_nucleo::TICK);
        if let Some(espera) = siguiente.checked_duration_since(Instant::now()) {
            std::thread::sleep(espera);
        } else {
            siguiente = Instant::now();
        }
    }
}
