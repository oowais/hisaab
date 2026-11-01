import { describe, expect, test } from 'vite-plus/test'
import { dueIn } from '../app/utils/days'

describe('dueIn', () => {
  const t = '2026-11-01'
  test.each([
    ['2026-11-01', 'today'],
    ['2026-11-02', 'tomorrow'],
    ['2026-11-03', 'in 2 days'],
    ['2026-11-27', 'in 26 days'],
    ['2027-01-01', 'in 2 months'],
    ['2027-11-01', 'in a year'],
    ['2026-10-31', 'yesterday'],
    ['2026-10-29', '3 days ago'],
  ])('%s → %s', (day, text) => expect(dueIn(day, t)).toBe(text))
})
