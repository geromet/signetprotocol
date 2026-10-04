import { Footer, Layout, Navbar } from 'nextra-theme-docs'
import { Banner, Head, Search } from 'nextra/components'
import { getPageMap } from 'nextra/page-map'
import 'nextra-theme-docs/style.css'
import './estilos.css'

export const metadata = {
  title: {
    default: 'Signet Protocol',
    template: '%s · Signet Protocol'
  },
  description: 'Every game, one world: an open protocol and SDK so different games can share a server, a world and a match.'
}

const logo = (
  <span className="mv-logo">
    <span className="mv-logo-marca" aria-hidden="true">◈</span>
    <b>Signet</b>
    <span className="mv-logo-sdk">PROTOCOL</span>
  </span>
)

export default async function RootLayout({ children }) {
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <Head color={{ hue: 265, saturation: 80 }} />
      <body>
        <Layout
          banner={<Banner storageKey="signet-beta-1">Signet SDK 0.1 is in beta: the API may change. Your translators are welcome!</Banner>}
          navbar={<Navbar logo={logo} />}
          pageMap={await getPageMap()}
          footer={<Footer>Apache-2.0 · {new Date().getFullYear()} · Signet Protocol. Games and their trademarks belong to their owners.</Footer>}
          sidebar={{ defaultMenuCollapseLevel: 1 }}
          editLink={null}
          feedback={{ content: null }}
          toc={{ title: 'On this page', backToTop: 'Back to top' }}
          search={<Search placeholder="Search the docs…" emptyResult="No results." errorText="Could not load the search index." loading="Loading…" />}
          copyPageButton={false}
        >
          {children}
        </Layout>
      </body>
    </html>
  )
}
