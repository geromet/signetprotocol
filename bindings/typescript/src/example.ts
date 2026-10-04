// Watches a server for 5 seconds and summarises what it sees.
//   npm run build && node dist/example.js 192.168.1.212

import { Client, OBSERVER, walkable } from "./index.js";

const host = process.argv[2] ?? "127.0.0.1";
const c = new Client(host, OBSERVER);
let deaths = 0;

c.on("welcome", (t, id, mode) => {
  const cells = t.alturas.length;
  const open = Array.from({ length: cells }, (_, k) => k).filter((k) => !(t.solido?.[k] ?? false)).length;
  console.log(`World ${t.fuente} (${t.lado}x${t.lado} m, ${open} walkable cells), mode ${mode}, I am #${id}`);
  console.log(`Can a body stand at the origin? ${walkable(t, 0, 0)}`);
});
c.on("event", (e) => {
  if ("Muerte" in e) deaths++;
});
setTimeout(() => {
  const humans = c.players.filter((j) => j.juego !== "bot");
  console.log(`${c.players.length} bodies (${humans.length} players: ${humans.map((j) => j.juego).join(", ") || "none"}), ${deaths} deaths in 5 s`);
  c.close();
}, 5000);
