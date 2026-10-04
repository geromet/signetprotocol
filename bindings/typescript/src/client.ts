// Signet/1 client for Node. The beta does not predict: it sends unnumbered
// intents (the server applies the latest one every tick), which is enough for
// bots, observers, server directories and tools. Prediction lives in the Rust
// SDK (and in C/C# through it) because it needs the game mode's exact rules.

import { createConnection, type Socket } from "node:net";
import { createInterface } from "node:readline";
import { EventEmitter } from "node:events";
import {
  PORT,
  PROTOCOL_VERSION,
  WEAPON_PISTOL,
  type DelCliente,
  type DelServidor,
  type Evento,
  type Intencion,
  type JugadorNet,
  type Terreno,
} from "./protocol.js";

export interface ClientEvents {
  welcome: [terrain: Terreno, myId: number, mode: string];
  state: [players: JugadorNet[]];
  event: [event: Evento];
  closed: [];
}

/** Friendly names for the fields of `Intencion`. */
export interface Intent {
  forward?: number;
  strafe?: number;
  yaw?: number;
  run?: boolean;
  fire?: boolean;
  weapon?: number;
}

export class Client extends EventEmitter<ClientEvents> {
  terrain?: Terreno;
  myId?: number;
  mode = "";
  players: JugadorNet[] = [];
  private socket: Socket;

  /** Connects to `host` ("192.168.1.212" or "host:port") as `game`. */
  constructor(host: string, game: string) {
    super();
    const [h, p] = host.includes(":") ? host.split(":") : [host, String(PORT)];
    this.socket = createConnection({ host: h, port: Number(p) }, () => {
      this.socket.setNoDelay(true);
      this.send({ Hola: { juego: game, version: PROTOCOL_VERSION } });
    });
    createInterface({ input: this.socket }).on("line", (line) => this.receive(line));
    this.socket.on("close", () => this.emit("closed"));
    this.socket.on("error", () => this.socket.destroy());
  }

  private receive(line: string) {
    let msg: DelServidor;
    try {
      msg = JSON.parse(line);
    } catch {
      return;
    }
    if ("Bienvenida" in msg) {
      const w = msg.Bienvenida;
      this.terrain = w.terreno;
      this.myId = w.tu_id;
      this.mode = w.modo ?? "";
      this.emit("welcome", w.terreno, w.tu_id, this.mode);
    } else if ("Estado" in msg) {
      this.players = msg.Estado.jugadores;
      this.emit("state", this.players);
      for (const e of msg.Estado.eventos ?? []) this.emit("event", e);
    }
  }

  /** Your own body in the latest snapshot. */
  me(): JugadorNet | undefined {
    return this.players.find((j) => j.id === this.myId);
  }

  /** Sends any raw Signet/1 message. */
  send(msg: DelCliente) {
    if (!this.socket.destroyed) this.socket.write(JSON.stringify(msg) + "\n");
  }

  /** What the player wants to do (unnumbered: the server uses it every tick until the next one). */
  intent(i: Intent) {
    const wire: Intencion = {
      avance: i.forward ?? 0,
      lateral: i.strafe ?? 0,
      yaw: i.yaw ?? 0,
      correr: i.run ?? false,
      disparar: i.fire ?? false,
      usar: false,
      arma: i.weapon ?? WEAPON_PISTOL,
    };
    this.send({ Intencion: wire });
  }

  close() {
    this.socket.end();
  }
}
