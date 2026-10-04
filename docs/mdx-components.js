import { useMDXComponents as getThemeComponents } from 'nextra-theme-docs'
import { GitHubStats } from './components/github-stats'
import { VisitStats } from './components/visitas'

const themeComponents = getThemeComponents()

export function useMDXComponents(components) {
  return { ...themeComponents, GitHubStats, VisitStats, ...components }
}
