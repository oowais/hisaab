// Pure helpers for the Timeline: where a subscription's price or plan
// changed, and how days map to pixels.
import type { Period } from '~/composables/api'
import { addDays, daysBetween, intervalText } from './days'
import { formatMoney } from './money'

export type Marker = { date: string; text: string }

const terms = (p: Period) =>
  [
    p.planLabel,
    `${formatMoney(p.amountMinor, p.currency)}${p.everyN === 1 && p.unit === 'month' ? '' : ` ${intervalText(p.everyN, p.unit)}`}`,
  ]
    .filter(Boolean)
    .join(' ')

/** A diamond wherever one paid stretch follows another with other terms:
 *  "Standard €13.99 → Basic €4.99". Pauses, trials and gaps aren't changes. */
export function priceChanges(periods: Period[]): Marker[] {
  const out: Marker[] = []
  const sorted = [...periods].sort((a, b) => a.startDate.localeCompare(b.startDate))
  for (let i = 1; i < sorted.length; i++) {
    const a = sorted[i - 1]!
    const b = sorted[i]!
    if (a.status !== 'active' || b.status !== 'active') continue
    if (!a.endDate || addDays(a.endDate, 1) !== b.startDate) continue
    const from = terms(a)
    const to = terms(b)
    if (from !== to) out.push({ date: b.startDate, text: `${from} → ${to}` })
  }
  return out
}

/** The days the chart spans: from the earliest start (or `today` minus a
 *  month) to a little after today or the latest scheduled change. */
export function domain(periods: Period[], today: string): { start: string; end: string } {
  let start = addDays(today, -30)
  let end = addDays(today, 45)
  for (const p of periods) {
    if (p.startDate < start) start = p.startDate
    const last = p.endDate ?? p.startDate
    if (addDays(last, 30) > end) end = addDays(last, 30)
  }
  return { start: addDays(start, -10), end }
}

/** x of the start of `day`, for a chart starting at `start`. */
export const xOf = (day: string, start: string, pxPerDay: number) => daysBetween(start, day) * pxPerDay
