import nextra from 'nextra'

const withNextra = nextra({
  defaultShowCopyCode: true,
  search: { codeblocks: false }
})

// Sitio estático (carpeta `out/`): se puede publicar en cualquier hosting.
export default withNextra({
  output: 'export',
  images: { unoptimized: true },
  trailingSlash: true
})
