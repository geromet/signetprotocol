//! El cliente más pequeño posible: entra al servidor, anda y gira durante unos
//! segundos e imprime dónde está y cuánto coincide con el servidor.
//!
//!   cargo run --example cliente_minimo -- 192.168.1.212 [segundos]

use std::time::{Duration, Instant};

use signet_sdk::cliente::{Cliente, EstadoConexion};
use signet_sdk::protocolo::Intencion;

fn main() {
    let host = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1".into());
    let segundos: f32 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(10.0);
    let mut c = Cliente::conectar(&host, "sdk-ejemplo");
    let inicio = Instant::now();
    let mut anterior = Instant::now();
    let mut ultimo_informe = Instant::now();
    let mut yaw = 0.0f32;
    while inicio.elapsed().as_secs_f32() < segundos {
        let dt = anterior.elapsed().as_secs_f32();
        anterior = Instant::now();
        if let Some(y) = c.tomar_yaw_aparicion() {
            yaw = y;
        }
        yaw += 0.5 * dt;
        c.fijar_intencion(Intencion { avance: 1.0, yaw, ..c.intencion() });
        let pos = c.avanzar(dt);
        if ultimo_informe.elapsed() > Duration::from_secs(1) {
            ultimo_informe = Instant::now();
            match (c.estado(), pos) {
                (EstadoConexion::Conectado, Some(p)) => {
                    let d = c.desincronia();
                    println!(
                        "[{}] {} | pos ({:6.2}, {:5.2}, {:6.2}) | commands {} sent, {} confirmed | corrections {} (max {:.2} m)",
                        c.modo(),
                        c.terreno().map_or(String::new(), |t| t.fuente.clone()),
                        p[0],
                        p[1],
                        p[2],
                        d.generado,
                        d.confirmado,
                        d.correcciones,
                        d.max_error
                    );
                }
                (e, _) => println!("{e:?}: {}", c.aviso()),
            }
            if std::env::var_os("MV_DEPURAR").is_some() {
                println!("  me = {:?}\n  intent = {:?}", c.yo(), c.intencion());
            }
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    let d = c.desincronia();
    println!("Done: {} commands confirmed, {} corrections, max error {:.3} m", d.confirmado, d.correcciones, d.max_error);
}
