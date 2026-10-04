'use client'

// Contador de visitas público: registra la página vista (sin cookies) y muestra las cifras.
// El servidor está en api/visits.js. En local o sin configurar, el contador simplemente no aparece.

import { useEffect, useState } from 'react'
import { usePathname } from 'next/navigation'

const API = '/api/visits'
const SITIOS = ['signetprotocol.io', 'www.signetprotocol.io']

function formato(n) {
  return new Intl.NumberFormat('en').format(n ?? 0)
}

function usarCifras(actualizarCon) {
  const [cifras, setCifras] = useState(null)
  useEffect(() => {
    let cancelado = false
    fetch(API)
      .then((r) => (r.ok ? r.json() : null))
      .then((datos) => !cancelado && datos && setCifras(datos))
      .catch(() => {})
    return () => {
      cancelado = true
    }
  }, [actualizarCon])
  return cifras
}

/** Cuenta la visita actual, como mucho una vez cada 30 minutos por página. */
function contarVisita(ruta) {
  try {
    if (!SITIOS.includes(location.hostname)) return Promise.resolve(false) // no contar desarrollo ni vistas previas
    if (navigator.doNotTrack === '1' || window.doNotTrack === '1') return Promise.resolve(false) // respeta "No rastrear"
    const ahora = Date.now()
    const ultima = JSON.parse(sessionStorage.getItem('signet-ultima') || 'null')
    if (ultima && ultima.ruta === ruta && ahora - ultima.t < 30 * 60 * 1000) return Promise.resolve(false)
    sessionStorage.setItem('signet-ultima', JSON.stringify({ ruta, t: ahora }))
    const nuevo = !localStorage.getItem('signet-visitante')
    if (nuevo) localStorage.setItem('signet-visitante', '1')
    return fetch(API, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: ruta, nuevo }),
      keepalive: true
    }).then(() => true, () => false)
  } catch {
    return Promise.resolve(false)
  }
}

/** Pie de página: registra la visita y muestra el total. */
export function VisitorCounter() {
  const ruta = usePathname()
  const [contada, setContada] = useState(0)
  useEffect(() => {
    let cancelado = false
    contarVisita(ruta).then((hecho) => !cancelado && hecho && setContada((n) => n + 1))
    return () => {
      cancelado = true
    }
  }, [ruta])
  const cifras = usarCifras(contada)
  if (!cifras) return null
  return (
    <a className="mv-visitas" href="/stats/" title="Public, cookie-free visit statistics">
      {formato(cifras.vistas)} visits
    </a>
  )
}

function nombrePais(codigo) {
  try {
    return new Intl.DisplayNames(['en'], { type: 'region' }).of(codigo) || codigo
  } catch {
    return codigo
  }
}

/** Página /stats: cifras, gráfico de 14 días, páginas y países. */
export function VisitStats() {
  const cifras = usarCifras(0)
  const [fallo, setFallo] = useState(false)
  useEffect(() => {
    const t = setTimeout(() => setFallo(true), 6000)
    return () => clearTimeout(t)
  }, [])

  if (!cifras) {
    return <p style={{ opacity: 0.7 }}>{fallo ? 'The statistics are not available right now.' : 'Loading…'}</p>
  }
  const maximo = Math.max(1, ...cifras.serie.map((d) => d.vistas))
  const tarjeta = (valor, etiqueta) => (
    <div className="mv-cifra">
      <span className="mv-cifra-valor">{formato(valor)}</span>
      <small>{etiqueta}</small>
    </div>
  )
  return (
    <div className="mv-stats">
      <div className="mv-cifras">
        {tarjeta(cifras.vistas, 'page views')}
        {tarjeta(cifras.unicos, 'unique browsers')}
        {tarjeta(cifras.hoy.vistas, 'views today (UTC)')}
        {tarjeta(cifras.siete_dias, 'views, last 7 days')}
      </div>

      <h3>Last 14 days</h3>
      <div className="mv-barras" role="img" aria-label="Page views per day, last 14 days">
        {cifras.serie.map((d) => (
          <div key={d.dia} className="mv-barra" title={`${d.dia}: ${formato(d.vistas)} views, ${formato(d.unicos)} new browsers`}>
            <span style={{ height: `${Math.max(2, (d.vistas / maximo) * 100)}%` }} />
            <small>{d.dia.slice(8)}</small>
          </div>
        ))}
      </div>

      <div className="mv-listas">
        <div>
          <h3>Top pages</h3>
          <table>
            <tbody>
              {cifras.paginas.map(([ruta, n]) => (
                <tr key={ruta}>
                  <td><a href={ruta}>{ruta}</a></td>
                  <td style={{ textAlign: 'right' }}>{formato(n)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div>
          <h3>Top countries</h3>
          <table>
            <tbody>
              {cifras.paises.map(([codigo, n]) => (
                <tr key={codigo}>
                  <td>{nombrePais(codigo)}</td>
                  <td style={{ textAlign: 'right' }}>{formato(n)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
      <p style={{ opacity: 0.6, fontSize: '0.85rem' }}>Updated {new Date(cifras.actualizado).toUTCString()}.</p>
    </div>
  )
}
