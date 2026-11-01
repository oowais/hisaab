// Calendar days as 'YYYY-MM-DD' strings in local time (no time zones: a
// charge is due on a day).

const pad = (n: number) => String(n).padStart(2, '0')

export const isoDay = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
export const parseDay = (s: string) => {
  const [y, m, d] = s.split('-').map(Number)
  return new Date(y!, m! - 1, d!)
}
export const todayIso = () => isoDay(new Date())
export const addDays = (s: string, n: number) => {
  const d = parseDay(s)
  d.setDate(d.getDate() + n)
  return isoDay(d)
}
/** Whole days from `a` to `b`. */
export const daysBetween = (a: string, b: string) => Math.round((parseDay(b).getTime() - parseDay(a).getTime()) / 86_400_000)

/** "today", "tomorrow", "in 3 days", "Mon 12 Jan" (past: "yesterday", "3 days ago"). */
export function relativeDay(s: string, today = todayIso()): string {
  const n = daysBetween(today, s)
  if (n === 0) return 'today'
  if (n === 1) return 'tomorrow'
  if (n === -1) return 'yesterday'
  if (n > 1 && n <= 6) return `in ${n} days`
  if (n < -1 && n >= -6) return `${-n} days ago`
  return shortDay(s, today)
}

/** "Mon 12 Jan", with the year when it isn't this year. */
export function shortDay(s: string, today = todayIso()): string {
  const d = parseDay(s)
  const sameYear = s.slice(0, 4) === today.slice(0, 4)
  return d.toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short', ...(sameYear ? {} : { year: 'numeric' }) })
}

/** "Jan 2025" (period lists). */
export const monthYear = (s: string) => parseDay(s).toLocaleDateString(undefined, { month: 'short', year: 'numeric' })

/** "monthly", "every 3 months", "yearly", "every 2 weeks". */
export function intervalText(everyN: number, unit: string): string {
  if (everyN === 1) return { day: 'daily', week: 'weekly', month: 'monthly', year: 'yearly' }[unit] ?? unit
  return `every ${everyN} ${unit}s`
}

/** "25 min", "2h", "3d" (sync and backup ages). */
export function span(ms: number): string {
  const MIN = 60_000
  const HOUR = 60 * MIN
  if (ms < HOUR) return `${Math.max(1, Math.round(ms / MIN))} min`
  if (ms < 24 * HOUR) return `${Math.round(ms / HOUR)}h`
  return `${Math.round(ms / (24 * HOUR))}d`
}

/** An instant as a day and time: "3 Nov, 21:30". */
export const instantLabel = (ms: number) =>
  new Date(ms).toLocaleString(undefined, { day: 'numeric', month: 'short', hour: 'numeric', minute: '2-digit' })
