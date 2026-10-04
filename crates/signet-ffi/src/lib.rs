//! C interface of the Signet SDK (see `include/signet.h`). It wraps
//! [`Cliente`] in `extern "C"` functions that never unwind across the
//! boundary: a null pointer or an error returns 0 / -1.

use std::ffi::{CStr, c_char};

use signet_sdk::cliente::{Cliente, EstadoConexion};
use signet_sdk::protocolo::Intencion;

/// A body in the server snapshot, laid out like `SgPlayer` in C.
#[repr(C)]
pub struct SgPlayer {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub health: i32,
    pub ammo: i32,
    pub frags: i32,
    pub weapon: u8,
    pub alive: u8,
    /// Game it joined with, NUL-terminated (truncated to 15 bytes).
    pub game: [c_char; 16],
}

fn text(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(p) }.to_str().ok().map(str::to_string)
}

/// Copies `s` into `buf` (NUL-terminated) and returns the length it needs without the NUL.
fn copy_out(s: &str, buf: *mut c_char, cap: usize) -> usize {
    if !buf.is_null() && cap > 0 {
        let n = s.len().min(cap - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(s.as_ptr() as *const c_char, buf, n);
            *buf.add(n) = 0;
        }
    }
    s.len()
}

/// SDK version, NUL-terminated (static).
#[unsafe(no_mangle)]
pub extern "C" fn sg_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

/// Connects to `host` ("192.168.1.212" or "host:port") as `game`. Never blocks.
#[unsafe(no_mangle)]
pub extern "C" fn sg_connect(host: *const c_char, game: *const c_char) -> *mut Cliente {
    let (Some(h), Some(g)) = (text(host), text(game)) else { return std::ptr::null_mut() };
    Box::into_raw(Box::new(Cliente::conectar(&h, &g)))
}

/// Closes the connection and frees the client.
#[unsafe(no_mangle)]
pub extern "C" fn sg_free(c: *mut Cliente) {
    if !c.is_null() {
        drop(unsafe { Box::from_raw(c) });
    }
}

fn client<'a>(c: *const Cliente) -> Option<&'a Cliente> {
    unsafe { c.as_ref() }
}

/// 0 connecting, 1 connected, 2 disconnected (or null pointer).
#[unsafe(no_mangle)]
pub extern "C" fn sg_state(c: *const Cliente) -> i32 {
    match client(c).map(Cliente::estado) {
        Some(EstadoConexion::Conectando) => 0,
        Some(EstadoConexion::Conectado) => 1,
        _ => 2,
    }
}

/// Id of your own body, or 0 if there is none yet.
#[unsafe(no_mangle)]
pub extern "C" fn sg_my_id(c: *const Cliente) -> u32 {
    client(c).and_then(Cliente::mi_id).unwrap_or(0)
}

/// Terrain side in 1 m cells, or 0 if it has not arrived yet.
#[unsafe(no_mangle)]
pub extern "C" fn sg_terrain_side(c: *const Cliente) -> i32 {
    client(c).and_then(Cliente::terreno).map_or(0, |t| t.lado as i32)
}

/// Floor height at neutral (x, z).
#[unsafe(no_mangle)]
pub extern "C" fn sg_terrain_height(c: *const Cliente, x: f32, z: f32) -> f32 {
    client(c).and_then(Cliente::terreno).map_or(0.0, |t| t.altura(x, z))
}

/// 1 if a body can stand at (x, z), 0 if it is a wall or outside the map.
#[unsafe(no_mangle)]
pub extern "C" fn sg_terrain_walkable(c: *const Cliente, x: f32, z: f32) -> i32 {
    client(c).and_then(Cliente::terreno).is_some_and(|t| t.transitable(x, z)) as i32
}

/// The whole terrain as Signet/1 JSON. Returns the bytes needed without the
/// NUL; if `cap` is too small the output is truncated: call again with more.
#[unsafe(no_mangle)]
pub extern "C" fn sg_terrain_json(c: *const Cliente, buf: *mut c_char, cap: usize) -> usize {
    let Some(t) = client(c).and_then(Cliente::terreno) else { return 0 };
    copy_out(&serde_json::to_string(&*t).unwrap_or_default(), buf, cap)
}

/// The native Signet map as JSON (lights, objects, sky), or 0 if the world is not native.
#[unsafe(no_mangle)]
pub extern "C" fn sg_native_map_json(c: *const Cliente, buf: *mut c_char, cap: usize) -> usize {
    let Some(m) = client(c).and_then(Cliente::mapa) else { return 0 };
    copy_out(&serde_json::to_string(&*m).unwrap_or_default(), buf, cap)
}

/// Copies up to `max` bodies into `out` and returns how many there are in total.
#[unsafe(no_mangle)]
pub extern "C" fn sg_players(c: *const Cliente, out: *mut SgPlayer, max: i32) -> i32 {
    let Some(c) = client(c) else { return 0 };
    let players = c.jugadores();
    if !out.is_null() {
        for (k, j) in players.iter().take(max.max(0) as usize).enumerate() {
            let mut game = [0 as c_char; 16];
            for (d, b) in game.iter_mut().zip(j.juego.bytes().take(15)) {
                *d = b as c_char;
            }
            unsafe {
                out.add(k).write(SgPlayer {
                    id: j.id,
                    x: j.pos[0],
                    y: j.pos[1],
                    z: j.pos[2],
                    yaw: j.yaw,
                    health: j.vida,
                    ammo: j.municion,
                    frags: j.frags,
                    weapon: j.arma,
                    alive: j.vivo as u8,
                    game,
                });
            }
        }
    }
    players.len() as i32
}

/// Events received since the last call, as a JSON array. Consumes them: if
/// `cap` is too small the ones that do not fit are lost, so use a generous buffer (64 KB).
#[unsafe(no_mangle)]
pub extern "C" fn sg_events_json(c: *const Cliente, buf: *mut c_char, cap: usize) -> usize {
    let Some(c) = client(c) else { return 0 };
    copy_out(&serde_json::to_string(&c.tomar_eventos()).unwrap_or_else(|_| "[]".into()), buf, cap)
}

/// What the player wants to do now: forward and strafe in [-1, 1], yaw in
/// radians (0 = -z, positive turns left), weapon 0 fist / 1 pistol / 2 shotgun.
#[unsafe(no_mangle)]
pub extern "C" fn sg_intent(c: *mut Cliente, forward: f32, strafe: f32, yaw: f32, run: i32, fire: i32, weapon: u8) {
    if let Some(c) = unsafe { c.as_mut() } {
        c.fijar_intencion(Intencion { avance: forward, lateral: strafe, yaw, correr: run != 0, disparar: fire != 0, arma: weapon, ..Default::default() });
    }
}

/// Advances `dt` seconds (prediction + sending). Writes where to draw the
/// player's feet into x, y, z and returns 1, or 0 if there is no body yet.
#[unsafe(no_mangle)]
pub extern "C" fn sg_advance(c: *mut Cliente, dt: f32, x: *mut f32, y: *mut f32, z: *mut f32) -> i32 {
    let Some(c) = (unsafe { c.as_mut() }) else { return 0 };
    let Some(p) = c.avanzar(dt) else { return 0 };
    unsafe {
        if !x.is_null() {
            *x = p[0];
        }
        if !y.is_null() {
            *y = p[1];
        }
        if !z.is_null() {
            *z = p[2];
        }
    }
    1
}

/// After a (re)spawn, writes the best direction to look at into `yaw` and returns 1.
#[unsafe(no_mangle)]
pub extern "C" fn sg_spawn_yaw(c: *mut Cliente, yaw: *mut f32) -> i32 {
    let Some(c) = (unsafe { c.as_mut() }) else { return 0 };
    match c.tomar_yaw_aparicion() {
        Some(v) if !yaw.is_null() => {
            unsafe { *yaw = v };
            1
        }
        _ => 0,
    }
}

/// Prediction corrections since connecting (0 = client and server agree).
#[unsafe(no_mangle)]
pub extern "C" fn sg_corrections(c: *const Cliente) -> u32 {
    client(c).map_or(0, |c| c.desincronia().correcciones)
}
