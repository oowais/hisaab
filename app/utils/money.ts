// Money for display. Amounts are integer minor units; the currency's
// decimals come from Intl (ISO 4217: JPY 0, EUR 2, KWD 3).

const digitsCache = new Map<string, number>()

export function minorDigits(currency: string): number {
  let d = digitsCache.get(currency)
  if (d == null) {
    try {
      d = new Intl.NumberFormat('en', { style: 'currency', currency }).resolvedOptions().maximumFractionDigits ?? 2
    } catch {
      d = 2
    }
    digitsCache.set(currency, d)
  }
  return d
}

/** "€9.99", "¥1,500"; `compact` drops ".00". */
export function formatMoney(minor: number, currency: string, opts: { compact?: boolean; locale?: string } = {}): string {
  const digits = minorDigits(currency)
  const value = minor / 10 ** digits
  const whole = opts.compact && Number.isInteger(value)
  try {
    return new Intl.NumberFormat(opts.locale, {
      style: 'currency',
      currency,
      minimumFractionDigits: whole ? 0 : digits,
      maximumFractionDigits: digits,
    }).format(value)
  } catch {
    return `${value.toFixed(digits)} ${currency}`
  }
}

/** Typed text ("9,99", "9.99", "1.500,50") → minor units, or null. */
export function parseAmount(text: string, currency: string): number | null {
  let s = text.trim().replace(/\s/g, '')
  if (!s) return null
  const lastComma = s.lastIndexOf(',')
  const lastDot = s.lastIndexOf('.')
  // The last separator is the decimal one when it has 1–3 digits after it
  // and the other separator (thousands) comes before it.
  const dec = Math.max(lastComma, lastDot)
  const decimals = dec >= 0 ? s.length - dec - 1 : 0
  const digits = minorDigits(currency)
  if (
    dec >= 0 &&
    (decimals <= digits || (lastComma >= 0 && lastDot >= 0)) &&
    decimals > 0 &&
    !(decimals === 3 && digits < 3 && (lastComma < 0 || lastDot < 0))
  ) {
    s = s.slice(0, dec).replace(/[.,]/g, '') + '.' + s.slice(dec + 1)
  } else {
    s = s.replace(/[.,]/g, '')
  }
  if (!/^\d+(\.\d+)?$/.test(s)) return null
  const n = Math.round(Number(s) * 10 ** digits)
  return Number.isFinite(n) ? n : null
}

/** Minor units → plain editable text ("9.99"). */
export const amountText = (minor: number, currency: string) => (minor / 10 ** minorDigits(currency)).toFixed(minorDigits(currency))

/** Common currencies first in pickers. */
export const COMMON_CURRENCIES = [
  'EUR',
  'USD',
  'GBP',
  'CHF',
  'INR',
  'PKR',
  'JPY',
  'CAD',
  'AUD',
  'SEK',
  'NOK',
  'DKK',
  'PLN',
  'CZK',
  'TRY',
  'AED',
  'SAR',
  'CNY',
]
