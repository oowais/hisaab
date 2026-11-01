// "Share → Hisaab" (Android): what Quick add can read from shared text, such
// as a link to an account page or a "your plan renews at €13.99" mail.
import { minorDigits, parseAmount } from './money'
import { TEMPLATES, type Template } from './templates'

export type Shared = {
  /** A known payment, found by the link's site or by name in the text. */
  template: Template | null
  /** The template's name, else the subject, the text's first line or the site. */
  name: string
  url: string | null
  amount: { minor: number; currency: string } | null
}

const SYMBOLS: Record<string, string> = { '€': 'EUR', $: 'USD', '£': 'GBP', '₹': 'INR', '¥': 'JPY', '₺': 'TRY', '₨': 'PKR' }
const NUMBER = String.raw`\d[\d.,]*\d|\d`
// "€13.99", "EUR 13,99", "13,99 €", "13.99 USD".
const AMOUNT = new RegExp(String.raw`([€$£₹¥₺₨]|\b[A-Z]{3}\b)\s?(${NUMBER})|(${NUMBER})\s?([€$£₹¥₺₨]|\b[A-Z]{3}\b)`)

/** "netflix.com" from "https://www.netflix.com/account" (last two labels). */
function site(url: string): string | null {
  try {
    return new URL(url).hostname.split('.').slice(-2).join('.').toLowerCase()
  } catch {
    return null
  }
}

let known: Set<string> | null = null
/** An ISO 4217 code this device knows ("ABC" or "THE" are not). */
function isCurrency(code: string) {
  known ??= new Set(Intl.supportedValuesOf('currency'))
  return known.has(code)
}

function findAmount(text: string): Shared['amount'] {
  const m = text.match(AMOUNT)
  if (!m) return null
  const mark = m[1] ?? m[4]!
  const currency = SYMBOLS[mark] ?? (isCurrency(mark) ? mark : null)
  if (!currency) return null
  const minor = parseAmount((m[2] ?? m[3]!).trim(), currency)
  // A year or a count, not a price: no decimals and implausibly big.
  if (minor == null || minor <= 0 || minor >= 10 ** (6 + minorDigits(currency))) return null
  return { minor, currency }
}

function findTemplate(text: string, url: string | null): Template | null {
  const s = url && site(url)
  if (s) {
    const bySite = TEMPLATES.find((t) => t.cancelUrl && site(t.cancelUrl) === s)
    if (bySite) return bySite
  }
  // Longest name first: "Apple TV+" before "Apple".
  const lower = ` ${text.toLowerCase()} `
  return (
    [...TEMPLATES]
      .sort((a, b) => b.name.length - a.name.length)
      .find((t) => {
        const i = lower.indexOf(t.name.toLowerCase())
        if (i < 0) return false
        const after = lower[i + t.name.length] ?? ' '
        return !/[\p{L}\p{N}]/u.test(lower[i - 1]!) && !/[\p{L}\p{N}]/u.test(after)
      }) ?? null
  )
}

/** "Proton" from "https://account.proton.me/…". */
function siteName(url: string): string {
  const s = site(url)?.split('.')[0] ?? ''
  return s.charAt(0).toUpperCase() + s.slice(1)
}

export function fromShare(text: string, subject = ''): Shared {
  const url = text.match(/https?:\/\/\S+/)?.[0] ?? null
  const rest = url ? text.replace(url, ' ') : text
  const all = `${subject}\n${rest}`
  const template = findTemplate(all, url)
  const firstLine = rest
    .split('\n')
    .map((l) => l.trim())
    .find(Boolean)
  const name = (template?.name ?? (subject.trim() || firstLine || (url ? siteName(url) : ''))).slice(0, 60)
  return { template, name, url, amount: findAmount(all) }
}
