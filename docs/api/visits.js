// Contador de visitas público de signetprotocol.io (función de Vercel).
//
//   POST /api/visits   registra una página vista  { path, nuevo }
//   GET  /api/visits   devuelve las cifras públicas
//
// Privacidad: sin cookies. No se guarda ninguna IP ni identificador: solo contadores
// agregados (total, por día, por página y por país). Para limitar abusos se usa un
// contador de 60 segundos con un hash de la IP y la fecha, que caduca solo.
//
// Almacén: Upstash Redis (integración de Vercel). Variables de entorno, con cualquiera de
// los dos nombres: UPSTASH_REDIS_REST_URL / UPSTASH_REDIS_REST_TOKEN o
// KV_REST_API_URL / KV_REST_API_TOKEN.

import { createHash } from 'node:crypto'

const ORIGENES = new Set(['signetprotocol.io', 'www.signetprotocol.io'])
const BOTS = /bot|crawl|spider|slurp|preview|facebookexternalhit|headless|lighthouse|python-requests|curl|wget|httpclient|monitor|uptime|axios|node-fetch|go-http/i
const RUTA_VALIDA = /^\/(?:[a-z0-9._-]+\/){0,4}$/i
const MAX_PAGINAS = 400
const LIMITE_POR_MINUTO = 40
const DIAS_GUARDADOS = 120
const DIAS_SERIE = 14

function configuracion() {
  const url = process.env.UPSTASH_REDIS_REST_URL || process.env.KV_REST_API_URL
  const token = process.env.UPSTASH_REDIS_REST_TOKEN || process.env.KV_REST_API_TOKEN
  return url && token ? { url: url.replace(/\/$/, ''), token } : null
}

async function redis(cfg, comandos) {
  const r = await fetch(`${cfg.url}/pipeline`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${cfg.token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify(comandos)
  })
  if (!r.ok) throw new Error(`redis ${r.status}`)
  const respuestas = await r.json()
  return respuestas.map((x) => {
    if (x.error) throw new Error(x.error)
    return x.result
  })
}

function dia(desplazamiento = 0) {
  return new Date(Date.now() - desplazamiento * 86400000).toISOString().slice(0, 10)
}

function pares(plano) {
  const salida = []
  for (let i = 0; i < (plano || []).length; i += 2) salida.push([plano[i], Number(plano[i + 1])])
  return salida.sort((a, b) => b[1] - a[1])
}

function hostDe(valor) {
  try {
    return new URL(valor).hostname
  } catch {
    return ''
  }
}

async function registrar(cfg, req, res) {
  // Solo se cuenta lo que llega desde la propia web.
  if (!ORIGENES.has(hostDe(req.headers.origin || req.headers.referer || ''))) return res.status(204).end()
  if (BOTS.test(req.headers['user-agent'] || '')) return res.status(204).end()

  const cuerpo = typeof req.body === 'string' ? JSON.parse(req.body || '{}') : req.body || {}
  const ruta = typeof cuerpo.path === 'string' && RUTA_VALIDA.test(cuerpo.path) ? cuerpo.path.toLowerCase() : null
  const nuevo = cuerpo.nuevo === true
  const hoy = dia()
  const pais = /^[A-Z]{2}$/.test(req.headers['x-vercel-ip-country'] || '') ? req.headers['x-vercel-ip-country'] : null

  // Límite de abuso: contador de 60 s con un hash que no se guarda más tiempo.
  const ip = String(req.headers['x-forwarded-for'] || '').split(',')[0].trim()
  const huella = createHash('sha256').update(`${ip}|${hoy}`).digest('hex').slice(0, 16)
  const [cuenta, existe, total] = await redis(cfg, [
    ['INCR', `sig:rl:${huella}`],
    ['HEXISTS', 'sig:paginas', ruta || '-'],
    ['HLEN', 'sig:paginas']
  ])
  await redis(cfg, [['EXPIRE', `sig:rl:${huella}`, 60]])
  if (cuenta > LIMITE_POR_MINUTO) return res.status(429).json({ ok: false })

  const caducidad = DIAS_GUARDADOS * 86400
  const comandos = [
    ['INCR', 'sig:vistas'],
    ['INCR', `sig:dia:${hoy}:vistas`],
    ['EXPIRE', `sig:dia:${hoy}:vistas`, caducidad]
  ]
  if (nuevo) {
    comandos.push(['INCR', 'sig:unicos'], ['INCR', `sig:dia:${hoy}:unicos`], ['EXPIRE', `sig:dia:${hoy}:unicos`, caducidad])
  }
  if (ruta && (existe === 1 || total < MAX_PAGINAS)) comandos.push(['HINCRBY', 'sig:paginas', ruta, 1])
  if (pais) comandos.push(['HINCRBY', 'sig:paises', pais, 1])
  await redis(cfg, comandos)
  return res.status(204).end()
}

async function leer(cfg, res) {
  const dias = Array.from({ length: DIAS_SERIE }, (_, i) => dia(DIAS_SERIE - 1 - i))
  const comandos = [
    ['GET', 'sig:vistas'],
    ['GET', 'sig:unicos'],
    ['HGETALL', 'sig:paginas'],
    ['HGETALL', 'sig:paises'],
    ...dias.flatMap((d) => [['GET', `sig:dia:${d}:vistas`], ['GET', `sig:dia:${d}:unicos`]])
  ]
  const r = await redis(cfg, comandos)
  const serie = dias.map((d, i) => ({ dia: d, vistas: Number(r[4 + i * 2] || 0), unicos: Number(r[5 + i * 2] || 0) }))
  const ultimo = serie[serie.length - 1]
  res.setHeader('Cache-Control', 'public, s-maxage=60, stale-while-revalidate=300')
  return res.status(200).json({
    vistas: Number(r[0] || 0),
    unicos: Number(r[1] || 0),
    hoy: { vistas: ultimo.vistas, unicos: ultimo.unicos },
    siete_dias: serie.slice(-7).reduce((a, d) => a + d.vistas, 0),
    serie,
    paginas: pares(r[2]).slice(0, 10),
    paises: pares(r[3]).slice(0, 10),
    actualizado: new Date().toISOString()
  })
}

export default async function handler(req, res) {
  const cfg = configuracion()
  if (!cfg) return res.status(503).json({ error: 'El contador todavía no está configurado.' })
  try {
    if (req.method === 'POST') return await registrar(cfg, req, res)
    if (req.method === 'GET') return await leer(cfg, res)
    res.setHeader('Allow', 'GET, POST')
    return res.status(405).end()
  } catch (error) {
    console.error('visits', error)
    return res.status(500).json({ error: 'No se pudo leer el contador.' })
  }
}
