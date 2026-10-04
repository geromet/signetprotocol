//! Pasa la suite de conformidad a un terreno: desde un archivo JSON exportado
//! por tu importador, o pidiéndoselo a un servidor en marcha.
//!
//!   cargo run --example conformidad -- mi_mapa.json
//!   cargo run --example conformidad -- --servidor 192.168.1.212

use std::time::Duration;

use signet_sdk::cliente::Cliente;
use signet_sdk::conformidad::{comprobar_reglas, comprobar_terreno};
use signet_sdk::protocolo::{OBSERVADOR, Terreno};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let terreno: Terreno = match args.as_slice() {
        [flag, host] if flag == "--servidor" => {
            let c = Cliente::conectar(host, OBSERVADOR);
            let mut t = None;
            for _ in 0..100 {
                if let Some(x) = c.terreno() {
                    t = Some((*x).clone());
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            t.unwrap_or_else(|| {
                eprintln!("the server did not send the terrain: {}", c.aviso());
                std::process::exit(2)
            })
        }
        [ruta] => serde_json::from_str(&std::fs::read_to_string(ruta).expect("read the file")).expect("Terreno JSON"),
        _ => {
            eprintln!("usage: conformidad <terrain.json> | --servidor <host>");
            std::process::exit(2)
        }
    };
    println!("{}: {}x{} cells, {} spawn points", terreno.fuente, terreno.lado, terreno.lado, terreno.apariciones.len());
    let fallos: Vec<_> = comprobar_terreno(&terreno).into_iter().chain(comprobar_reglas(&terreno)).collect();
    if fallos.is_empty() {
        println!("CONFORMANT: passes T01–T07 and R01–R02");
    } else {
        for f in &fallos {
            println!("FAIL {}: {}", f.codigo, f.mensaje);
        }
        std::process::exit(1);
    }
}
