import { describe, expect, test } from 'vite-plus/test'
import type { SubscriptionView } from '../app/composables/api'
import { isFiltered, matches, NO_CATEGORY, NO_FILTER } from '../app/utils/homeFilter'

const sub = (kind: 'subscription' | 'bill', categoryId: string | null) => ({ kind, categoryId }) as SubscriptionView
const known = new Map([['a', 'Streaming']])

describe('home filter', () => {
  test('kind and categories', () => {
    expect(matches({ kind: 'all', categories: [] }, sub('bill', null), known)).toBe(true)
    expect(matches({ kind: 'bill', categories: [] }, sub('subscription', 'a'), known)).toBe(false)
    expect(matches({ kind: 'all', categories: ['a'] }, sub('bill', 'a'), known)).toBe(true)
    expect(matches({ kind: 'all', categories: ['a'] }, sub('bill', null), known)).toBe(false)
    expect(matches({ kind: 'all', categories: [NO_CATEGORY] }, sub('bill', null), known)).toBe(true)
  })
  test('a deleted category is no category, and no longer filters', () => {
    expect(matches({ kind: 'all', categories: [NO_CATEGORY] }, sub('bill', 'gone'), known)).toBe(true)
    expect(matches({ kind: 'all', categories: ['gone'] }, sub('bill', 'a'), known)).toBe(true)
  })
  test('hiding ended and archived counts as a filter', () => {
    expect(isFiltered(NO_FILTER)).toBe(false)
    expect(isFiltered({ ...NO_FILTER, hideOver: true })).toBe(true)
    // Saved before the option existed.
    expect(isFiltered({ kind: 'all', categories: [] })).toBe(false)
  })
})
