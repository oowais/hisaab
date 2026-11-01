// Home's filter: a kind and any categories (none = all of them).
import type { Kind, SubscriptionView } from '~/composables/api'

export type HomeFilter = { kind: Kind | 'all'; categories: string[] }
/** The "No category" entry's key. */
export const NO_CATEGORY = 'none'

export function matches(f: HomeFilter, s: SubscriptionView, known: Map<string, string>): boolean {
  if (f.kind !== 'all' && s.kind !== f.kind) return false
  // A category deleted since (or not loaded yet) doesn't filter.
  const cats = f.categories.filter((c) => c === NO_CATEGORY || known.has(c))
  if (!cats.length) return true
  return cats.includes(s.categoryId && known.has(s.categoryId) ? s.categoryId : NO_CATEGORY)
}
