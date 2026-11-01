// Pre-made categories: offered as chips next to your own and saved only when
// first picked (core `ensure_category`). Fixed ids, so two devices that pick
// "Streaming" end up with one category after syncing. Never change an id.
import type { Category } from '~/composables/api'

export type Suggested = { key: string; id: string; name: string; color: string }

const id = (n: number) => `0190f0a0-0000-7000-8000-${String(n).padStart(12, '0')}`

export const SUGGESTED: Suggested[] = [
  { key: 'streaming', id: id(1), name: 'Streaming', color: '#e0644f' },
  { key: 'music', id: id(2), name: 'Music', color: '#c4507d' },
  { key: 'news', id: id(3), name: 'News and reading', color: '#c48a2c' },
  { key: 'software', id: id(4), name: 'Software', color: '#5b6ee1' },
  { key: 'cloud', id: id(5), name: 'Cloud and storage', color: '#2f8fb0' },
  { key: 'gaming', id: id(6), name: 'Gaming', color: '#9a5bc4' },
  { key: 'health', id: id(7), name: 'Health and fitness', color: '#3f9a6e' },
  { key: 'education', id: id(8), name: 'Education', color: '#ffb319' },
  { key: 'shopping', id: id(9), name: 'Shopping and food', color: '#e50914' },
  { key: 'transport', id: id(10), name: 'Transport', color: '#6b7a8f' },
  { key: 'phone', id: id(11), name: 'Phone and internet', color: '#1ed760' },
  { key: 'housing', id: id(12), name: 'Housing', color: '#c48a2c' },
  { key: 'utilities', id: id(13), name: 'Utilities', color: '#2f8fb0' },
  { key: 'insurance', id: id(14), name: 'Insurance', color: '#5b6ee1' },
  { key: 'taxes', id: id(15), name: 'Taxes and fees', color: '#6b7a8f' },
  { key: 'loans', id: id(16), name: 'Loans', color: '#9a5bc4' },
]

/** A template's usual category (Quick add picks it with the template). */
const BY_TEMPLATE: Record<string, string[]> = {
  streaming: [
    'netflix',
    'youtube-premium',
    'disney-plus',
    'prime',
    'apple-tv',
    'paramount-plus',
    'max',
    'dazn',
    'sky',
    'rtl-plus',
    'crunchyroll',
    'youtube-tv',
    'hulu',
    'peacock',
    'fubo',
    'wow',
    'magenta-tv',
  ],
  music: ['spotify', 'apple-music', 'deezer', 'tidal', 'youtube-music', 'siriusxm', 'audible'],
  news: ['nytimes', 'spiegel', 'medium', 'substack', 'patreon'],
  cloud: ['icloud', 'google-one', 'proton', 'dropbox', 'backblaze', 'hetzner'],
  software: [
    'claude',
    'chatgpt',
    'gemini',
    'microsoft-365',
    'adobe',
    'canva',
    'linkedin',
    'zoom',
    'grammarly',
    'todoist',
    'discord',
    'cursor',
    'github',
    'jetbrains',
    'notion',
    'obsidian',
    '1password',
    'bitwarden',
    'kagi',
    'nordvpn',
    'mullvad',
    'surfshark',
    'ring',
  ],
  gaming: ['playstation-plus', 'nintendo-online', 'xbox-game-pass', 'chess'],
  health: ['strava', 'peloton', 'fitbit', 'headspace', 'gym'],
  education: ['duolingo'],
  shopping: ['hellofresh', 'doordash', 'instacart', 'walmart-plus', 'costco'],
  transport: ['uber-one', 'bahncard', 'adac', 'deutschlandticket'],
  phone: ['vodafone', 'o2', 'telekom', '1und1', 'freenet', 'verizon', 't-mobile', 'att', 'spectrum', 'phone', 'internet'],
  housing: ['rent'],
  utilities: ['electricity', 'gas', 'water'],
  insurance: ['haftpflicht', 'hausrat', 'renters-insurance', 'health-insurance', 'car-insurance'],
  taxes: ['rundfunkbeitrag', 'kfz-steuer', 'bank-account'],
  loans: ['student-loan', 'car-loan'],
}

export function suggestedFor(templateKey: string | null | undefined): Suggested | null {
  if (!templateKey) return null
  const key = Object.keys(BY_TEMPLATE).find((k) => BY_TEMPLATE[k]!.includes(templateKey))
  return SUGGESTED.find((s) => s.key === key) ?? null
}

/**
 * Chips: your categories, then the pre-made ones you don't have yet (not by
 * id, nor by name: a "Music" you made yourself hides the suggestion).
 */
export function categoryChips(mine: Category[]): (Category | Suggested)[] {
  const ids = new Set(mine.map((c) => c.id))
  const names = new Set(mine.map((c) => c.name.trim().toLowerCase()))
  return [...mine, ...SUGGESTED.filter((s) => !ids.has(s.id) && !names.has(s.name.toLowerCase()))]
}

export const isSuggested = (c: Category | Suggested): c is Suggested => 'key' in c
