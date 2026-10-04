'use client'

// Sustituye a `@theguild/remark-mermaid/mermaid` (ver next.config.mjs).
//
// La versión original vuelve a dibujar el diagrama con el MISMO id en cada cambio de
// atributos de <html> (Shift, una extensión, el selector de tema...). Mermaid borra
// primero el <svg> que ya estaba en pantalla y, como el resultado es idéntico, React no
// lo vuelve a pintar: el diagrama desaparece. Aquí:
//   * cada dibujo usa un id nuevo,
//   * solo se redibuja si cambia el tema claro/oscuro o el texto del diagrama,
//   * si algo falla se conserva el último dibujo bueno.

import { useEffect, useId, useRef, useState } from 'react'

// Tema monocromo: solo grises, para que encaje con el resto de la web.
const TEMAS = {
  light: {
    background: 'transparent',
    primaryColor: '#f5f5f5',
    primaryTextColor: '#0a0a0a',
    primaryBorderColor: '#a3a3a3',
    secondaryColor: '#e5e5e5',
    tertiaryColor: '#fafafa',
    lineColor: '#525252',
    textColor: '#0a0a0a',
    noteBkgColor: '#f5f5f5',
    noteTextColor: '#0a0a0a',
    noteBorderColor: '#a3a3a3',
    actorBkg: '#f5f5f5',
    actorBorder: '#a3a3a3',
    actorTextColor: '#0a0a0a',
    actorLineColor: '#a3a3a3',
    signalColor: '#525252',
    signalTextColor: '#0a0a0a'
  },
  dark: {
    background: 'transparent',
    primaryColor: '#1c1c1c',
    primaryTextColor: '#fafafa',
    primaryBorderColor: '#737373',
    secondaryColor: '#262626',
    tertiaryColor: '#171717',
    lineColor: '#a3a3a3',
    textColor: '#fafafa',
    noteBkgColor: '#1c1c1c',
    noteTextColor: '#fafafa',
    noteBorderColor: '#737373',
    actorBkg: '#1c1c1c',
    actorBorder: '#737373',
    actorTextColor: '#fafafa',
    actorLineColor: '#737373',
    signalColor: '#a3a3a3',
    signalTextColor: '#fafafa'
  }
}

function esOscuro() {
  const html = document.documentElement
  return html.classList.contains('dark') || html.getAttribute('data-theme') === 'dark'
}

function useVisible(ref) {
  const [visible, setVisible] = useState(false)
  useEffect(() => {
    const el = ref.current
    if (!el) return
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) {
        observer.disconnect()
        setVisible(true)
      }
    })
    observer.observe(el)
    return () => observer.disconnect()
  }, [ref])
  return visible
}

export function Mermaid({ chart }) {
  const base = useId().replaceAll(':', '')
  const contenedor = useRef(null)
  const visible = useVisible(contenedor)
  const [svg, setSvg] = useState('')
  const [oscuro, setOscuro] = useState(null)
  const contador = useRef(0)

  // Solo reaccionamos a un cambio REAL de tema, no a cualquier atributo de <html>.
  useEffect(() => {
    setOscuro(esOscuro())
    const observer = new MutationObserver(() => {
      const ahora = esOscuro()
      setOscuro((antes) => (antes === ahora ? antes : ahora))
    })
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'data-theme'] })
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    if (!visible || oscuro === null) return
    let cancelado = false
    ;(async () => {
      try {
        const { default: mermaid } = await import('mermaid')
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: 'loose',
          fontFamily: 'inherit',
          themeCSS: 'margin: 1.5rem auto 0;',
          theme: 'base',
          themeVariables: TEMAS[oscuro ? 'dark' : 'light']
        })
        const id = `${base}-${++contador.current}`
        const { svg: nuevo } = await mermaid.render(id, chart.replaceAll('\\n', '\n'))
        if (!cancelado) setSvg(nuevo)
      } catch (error) {
        console.error('Error while rendering mermaid', error)
      }
    })()
    return () => {
      cancelado = true
    }
  }, [chart, visible, oscuro, base])

  return <div ref={contenedor} dangerouslySetInnerHTML={{ __html: svg }} />
}
