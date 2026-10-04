import path from 'node:path'
import { fileURLToPath } from 'node:url'
import nextra from 'nextra'

const aqui = path.dirname(fileURLToPath(import.meta.url))

const withNextra = nextra({
  defaultShowCopyCode: true,
  search: { codeblocks: false }
})

// Sitio estático (carpeta `out/`): se puede publicar en cualquier hosting.
export default withNextra({
  output: 'export',
  images: { unoptimized: true },
  trailingSlash: true,
  // Los diagramas usan nuestro componente en lugar del de la librería: ver components/mermaid.jsx.
  webpack(config) {
    config.resolve.alias['@theguild/remark-mermaid/mermaid$'] = path.join(aqui, 'components', 'mermaid.jsx')
    return config
  }
})
