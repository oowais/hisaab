import { expect, test } from 'vite-plus/test'
import type { Period } from '../app/composables/api'
import { domain, priceChanges } from '../app/utils/timeline'

const p = (
  id: string,
  status: Period['status'],
  amount: number,
  start: string,
  end: string | null,
  plan: string | null = null,
): Period => ({
  id,
  subscriptionId: 's',
  status,
  amountMinor: amount,
  currency: 'EUR',
  everyN: 1,
  unit: 'month',
  planLabel: plan,
  amountKind: 'fixed',
  anchorDate: start,
  startDate: start,
  endDate: end,
})

test('price and plan changes between adjacent paid periods', () => {
  const ms = priceChanges([
    p('b', 'active', 499, '2026-02-05', '2026-05-31', 'Basic'),
    p('a', 'active', 1399, '2025-03-05', '2026-02-04', 'Standard'),
    p('c', 'paused', 499, '2026-06-01', '2026-06-30', 'Basic'),
    p('d', 'active', 499, '2026-07-01', null, 'Basic'),
  ])
  expect(ms).toEqual([{ date: '2026-02-05', text: 'Standard €13.99 → Basic €4.99' }])
})

test('no marker across gaps or for identical terms', () => {
  expect(priceChanges([p('a', 'active', 100, '2026-01-01', '2026-01-31'), p('b', 'active', 100, '2026-02-01', null)])).toEqual([])
  expect(priceChanges([p('a', 'active', 100, '2026-01-01', '2026-01-20'), p('b', 'active', 200, '2026-02-01', null)])).toEqual([])
})

test('domain covers history and the future', () => {
  const d = domain([p('a', 'active', 1, '2024-05-01', null), p('b', 'active', 1, '2027-01-01', '2027-03-01')], '2026-11-01')
  expect(d).toEqual({ start: '2024-04-21', end: '2027-03-31' })
})
