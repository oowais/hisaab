// Home's filter: a kind, any categories (none = all of them), and whether
// ended and archived payments show at the bottom.
import type { Kind, SubscriptionView } from '~/composables/api'

export type HomeFilter = { kind: Kind | 'all'; categories: string[]; hideOver?: boolean }
/** The "No category" entry's key. */
export const NO_CATEGORY = 'none'

export const isFiltered = (f: HomeFilter) => f.kind !== 'all' || f.categories.length > 0 || !!f.hideOver
export const NO_FILTER: HomeFilter = { kind: 'all', categories: [], hideOver: false }

export function matches(f: HomeFilter, s: SubscriptionView, known: Map<string, string>): boolean {
  if (f.kind !== 'all' && s.kind !== f.kind) return false
  // A category deleted since (or not loaded yet) doesn't filter.
  const cats = f.categories.filter((c) => c === NO_CATEGORY || known.has(c))
  if (!cats.length) return true
  return cats.includes(s.categoryId && known.has(s.categoryId) ? s.categoryId : NO_CATEGORY)
}
