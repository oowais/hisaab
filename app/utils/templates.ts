// Quick add templates: icon, colour and usual billing for common payments.
// No prices: they differ by country and plan, and go stale.
// `currency` only for payments that exist in one currency (German bills);
// others use the last currency used.
import type { Kind, Unit } from '~/composables/api'

export type Template = {
  key: string
  name: string
  kind: Kind
  icon: string
  color: string
  currency?: string
  every?: number
  unit?: Unit
  /** Variable bills (electricity): the amount is an estimate. */
  estimated?: boolean
  /** Extra words that should find it ("tv", "music"). */
  tags?: string
}

const sub = (key: string, name: string, icon: string, color: string, extra: Partial<Template> = {}): Template => ({
  key,
  name,
  kind: 'subscription',
  icon,
  color,
  ...extra,
})
const bill = (key: string, name: string, icon: string, color: string, extra: Partial<Template> = {}): Template => ({
  key,
  name,
  kind: 'bill',
  icon,
  color,
  ...extra,
})

export const TEMPLATES: Template[] = [
  // Streaming, music, books
  sub('netflix', 'Netflix', 'si:netflix', '#E50914', { tags: 'tv streaming' }),
  sub('spotify', 'Spotify', 'si:spotify', '#1ED760', { tags: 'music' }),
  sub('youtube-premium', 'YouTube Premium', 'si:youtube', '#FF0000', { tags: 'tv music' }),
  sub('disney-plus', 'Disney+', 'lu:Film', '#113CCF', { tags: 'tv streaming' }),
  sub('prime', 'Amazon Prime', 'lu:Package', '#00A8E1', { tags: 'tv streaming video', unit: 'year' }),
  sub('apple-tv', 'Apple TV+', 'si:appletv', '#000000', { tags: 'tv streaming' }),
  sub('apple-music', 'Apple Music', 'si:applemusic', '#FA243C', { tags: 'music' }),
  sub('paramount-plus', 'Paramount+', 'si:paramountplus', '#0064FF', { tags: 'tv streaming' }),
  sub('max', 'HBO Max', 'si:max', '#002BE7', { tags: 'tv streaming' }),
  sub('dazn', 'DAZN', 'si:dazn', '#F8F8F5', { tags: 'sport tv' }),
  sub('sky', 'Sky', 'si:sky', '#0072C9', { tags: 'tv sport wow' }),
  sub('rtl-plus', 'RTL+', 'si:rtl', '#FA002E', { tags: 'tv' }),
  sub('crunchyroll', 'Crunchyroll', 'si:crunchyroll', '#FF5E00', { tags: 'anime tv' }),
  sub('deezer', 'Deezer', 'si:deezer', '#A238FF', { tags: 'music' }),
  sub('tidal', 'Tidal', 'si:tidal', '#000000', { tags: 'music' }),
  sub('audible', 'Audible', 'si:audible', '#F8991C', { tags: 'books audiobooks' }),
  sub('youtube-music', 'YouTube Music', 'si:youtubemusic', '#FF0000', { tags: 'music' }),
  sub('youtube-tv', 'YouTube TV', 'si:youtubetv', '#FF0000', { tags: 'tv live streaming' }),
  sub('hulu', 'Hulu', 'lu:Tv', '#1CE783', { tags: 'tv streaming' }),
  sub('peacock', 'Peacock', 'lu:Tv', '#000000', { tags: 'tv streaming nbc' }),
  sub('fubo', 'Fubo', 'si:fubo', '#C83D1E', { tags: 'tv sport live' }),
  sub('siriusxm', 'SiriusXM', 'lu:Radio', '#0000EB', { tags: 'radio music' }),
  sub('wow', 'WOW', 'lu:Tv', '#000000', { tags: 'tv streaming sky' }),
  sub('magenta-tv', 'MagentaTV', 'si:deutschetelekom', '#E20074', { tags: 'tv streaming telekom' }),
  sub('nytimes', 'New York Times', 'si:newyorktimes', '#000000', { tags: 'news newspaper nyt' }),
  sub('spiegel', 'Spiegel+', 'lu:Newspaper', '#E64415', { tags: 'news zeitung' }),
  sub('medium', 'Medium', 'si:medium', '#000000', { tags: 'reading blog' }),
  sub('substack', 'Substack', 'si:substack', '#FF6719', { tags: 'newsletter reading' }),
  // Cloud and storage
  sub('icloud', 'iCloud+', 'si:icloud', '#3693F3', { tags: 'storage apple' }),
  sub('google-one', 'Google One', 'si:google', '#4285F4', { tags: 'storage drive' }),
  sub('proton', 'Proton', 'si:proton', '#6D4AFF', { tags: 'mail vpn drive pass' }),
  sub('dropbox', 'Dropbox', 'si:dropbox', '#0061FF', { tags: 'storage' }),
  sub('backblaze', 'Backblaze', 'si:backblaze', '#E21E29', { tags: 'backup storage' }),
  sub('hetzner', 'Hetzner', 'si:hetzner', '#D50C2D', { tags: 'server hosting' }),
  // Tools, AI, development
  sub('claude', 'Claude', 'si:claude', '#D97757', { tags: 'ai anthropic' }),
  sub('chatgpt', 'ChatGPT', 'lu:Bot', '#10A37F', { tags: 'ai openai' }),
  sub('gemini', 'Google Gemini', 'si:googlegemini', '#8E75B2', { tags: 'ai google' }),
  sub('microsoft-365', 'Microsoft 365', 'lu:Briefcase', '#D83B01', { tags: 'office word excel onedrive', unit: 'year' }),
  sub('adobe', 'Adobe Creative Cloud', 'lu:Sparkles', '#FA0F00', { tags: 'photoshop lightroom' }),
  sub('canva', 'Canva', 'lu:Sparkles', '#00C4CC', { tags: 'design' }),
  sub('linkedin', 'LinkedIn Premium', 'lu:Briefcase', '#0A66C2', { tags: 'jobs career' }),
  sub('zoom', 'Zoom', 'si:zoom', '#0B5CFF', { tags: 'video calls' }),
  sub('grammarly', 'Grammarly', 'si:grammarly', '#027E6F', { tags: 'writing' }),
  sub('todoist', 'Todoist', 'si:todoist', '#E44332', { tags: 'tasks todo' }),
  sub('discord', 'Discord Nitro', 'si:discord', '#5865F2', { tags: 'chat games' }),
  sub('cursor', 'Cursor', 'si:cursor', '#000000', { tags: 'ai code' }),
  sub('github', 'GitHub', 'si:github', '#181717', { tags: 'code copilot' }),
  sub('jetbrains', 'JetBrains', 'si:jetbrains', '#000000', { tags: 'code ide', unit: 'year' }),
  sub('notion', 'Notion', 'si:notion', '#000000', { tags: 'notes' }),
  sub('obsidian', 'Obsidian Sync', 'si:obsidian', '#7C3AED', { tags: 'notes' }),
  sub('1password', '1Password', 'si:1password', '#145FE4', { tags: 'passwords', unit: 'year' }),
  sub('bitwarden', 'Bitwarden', 'si:bitwarden', '#175DDC', { tags: 'passwords', unit: 'year' }),
  sub('kagi', 'Kagi', 'si:kagi', '#FFB319', { tags: 'search' }),
  sub('nordvpn', 'NordVPN', 'si:nordvpn', '#4687FF', { tags: 'vpn', unit: 'year' }),
  sub('mullvad', 'Mullvad VPN', 'si:mullvad', '#294D73', { tags: 'vpn' }),
  sub('surfshark', 'Surfshark', 'si:surfshark', '#1EBFBF', { tags: 'vpn', unit: 'year' }),
  // Games, fitness, learning, creators
  sub('playstation-plus', 'PlayStation Plus', 'si:playstation', '#0070D1', { tags: 'games ps', unit: 'year' }),
  sub('nintendo-online', 'Nintendo Switch Online', 'lu:Gamepad2', '#E60012', { tags: 'games', unit: 'year' }),
  sub('xbox-game-pass', 'Xbox Game Pass', 'lu:Gamepad2', '#107C10', { tags: 'games microsoft' }),
  sub('chess', 'Chess.com', 'si:chessdotcom', '#81B64C', { tags: 'games' }),
  sub('strava', 'Strava', 'si:strava', '#FC4C02', { tags: 'fitness running' }),
  sub('peloton', 'Peloton', 'si:peloton', '#181A1D', { tags: 'fitness bike' }),
  sub('fitbit', 'Fitbit Premium', 'si:fitbit', '#00B0B9', { tags: 'fitness health' }),
  sub('headspace', 'Headspace', 'si:headspace', '#F47D31', { tags: 'meditation sleep' }),
  sub('duolingo', 'Duolingo', 'si:duolingo', '#58CC02', { tags: 'language learning', unit: 'year' }),
  sub('patreon', 'Patreon', 'si:patreon', '#000000', { tags: 'creator support' }),
  sub('gym', 'Gym', 'lu:Dumbbell', '#3F9A6E', { tags: 'fitness studio' }),
  // Shopping, food, transport
  sub('hellofresh', 'HelloFresh', 'si:hellofresh', '#99CC33', { tags: 'food meal kit', unit: 'week' }),
  sub('doordash', 'DashPass', 'si:doordash', '#FF3008', { tags: 'doordash food delivery' }),
  sub('uber-one', 'Uber One', 'si:uber', '#000000', { tags: 'uber eats rides delivery' }),
  sub('instacart', 'Instacart+', 'si:instacart', '#43B02A', { tags: 'groceries delivery', unit: 'year' }),
  sub('walmart-plus', 'Walmart+', 'lu:ShoppingCart', '#0071DC', { tags: 'shopping delivery', unit: 'year' }),
  sub('costco', 'Costco', 'lu:ShoppingCart', '#E31837', { tags: 'membership shopping', unit: 'year' }),
  sub('bahncard', 'BahnCard', 'si:deutschebahn', '#F01414', { currency: 'EUR', unit: 'year', tags: 'train db bahn' }),
  sub('adac', 'ADAC', 'lu:Car', '#FFCC00', { currency: 'EUR', unit: 'year', tags: 'auto car breakdown' }),
  sub('ring', 'Ring Protect', 'si:ring', '#1C9AD6', { tags: 'camera doorbell security' }),
  // Phone and internet
  sub('vodafone', 'Vodafone', 'si:vodafone', '#E60000', { tags: 'phone mobile internet' }),
  sub('o2', 'O2', 'si:o2', '#0050FF', { tags: 'phone mobile' }),
  sub('telekom', 'Telekom', 'si:deutschetelekom', '#E20074', { tags: 'phone mobile internet magenta' }),
  sub('1und1', '1&1', 'si:1and1', '#003D8F', { tags: 'phone mobile internet dsl' }),
  sub('freenet', 'freenet', 'si:freenet', '#84BC34', { tags: 'phone mobile' }),
  sub('verizon', 'Verizon', 'si:verizon', '#CD040B', { tags: 'phone mobile internet fios' }),
  sub('t-mobile', 'T-Mobile', 'lu:Smartphone', '#E20074', { tags: 'phone mobile' }),
  sub('att', 'AT&T', 'lu:Smartphone', '#00A8E0', { tags: 'phone mobile internet' }),
  sub('spectrum', 'Spectrum', 'si:spectrum', '#7B16FF', { tags: 'internet cable tv' }),
  bill('phone', 'Phone plan', 'lu:Smartphone', '#5B6EE1', { tags: 'mobile handy' }),
  bill('internet', 'Internet', 'lu:Wifi', '#2F8FB0', { tags: 'dsl fiber wlan' }),
  // Home, insurance, taxes
  bill('deutschlandticket', 'Deutschlandticket', 'lu:TrainFront', '#E30613', { currency: 'EUR', tags: 'train transport bahn' }),
  bill('rent', 'Rent', 'lu:House', '#6B7A8F', { tags: 'miete home' }),
  bill('electricity', 'Electricity', 'lu:Zap', '#C48A2C', { estimated: true, tags: 'strom power' }),
  bill('gas', 'Gas', 'lu:Flame', '#E0644F', { estimated: true, tags: 'heating heizung' }),
  bill('water', 'Water', 'lu:Droplet', '#2F8FB0', { estimated: true, tags: 'wasser utilities' }),
  bill('rundfunkbeitrag', 'Rundfunkbeitrag', 'lu:Radio', '#4E5D6C', { currency: 'EUR', every: 3, unit: 'month', tags: 'gez tv radio' }),
  bill('haftpflicht', 'Haftpflichtversicherung', 'lu:Shield', '#3F7CAC', { currency: 'EUR', unit: 'year', tags: 'insurance liability' }),
  bill('hausrat', 'Hausratversicherung', 'lu:Sofa', '#9A5BC4', { currency: 'EUR', unit: 'year', tags: 'insurance contents' }),
  bill('health-insurance', 'Health insurance', 'lu:HeartPulse', '#C4507D', { tags: 'krankenversicherung' }),
  bill('car-insurance', 'Car insurance', 'lu:Car', '#2F8FB0', { unit: 'year', tags: 'kfz auto' }),
  bill('kfz-steuer', 'Kfz-Steuer', 'lu:Car', '#6B7A8F', { currency: 'EUR', unit: 'year', tags: 'car tax auto' }),
  bill('renters-insurance', "Renter's insurance", 'lu:House', '#3F7CAC', { tags: 'insurance home' }),
  bill('student-loan', 'Student loan', 'lu:GraduationCap', '#5B6EE1', { tags: 'loan debt' }),
  bill('car-loan', 'Car payment', 'lu:Car', '#9A5BC4', { tags: 'loan lease auto' }),
  bill('bank-account', 'Bank account fee', 'lu:Landmark', '#6B7A8F', { tags: 'konto kontoführung sparkasse n26' }),
]

/** Best matches for `q` (name first, then tags); all when empty. */
export function searchTemplates(q: string, limit = 8): Template[] {
  const s = q.trim().toLowerCase()
  if (!s) return TEMPLATES.slice(0, limit)
  const score = (t: Template) => {
    const name = t.name.toLowerCase()
    if (name.startsWith(s)) return 0
    if (name.split(/[\s+.-]/).some((w) => w.startsWith(s))) return 1
    if (s.length >= 3 && name.includes(s)) return 2
    if (t.tags?.split(' ').some((w) => w.startsWith(s))) return 3
    return -1
  }
  return TEMPLATES.map((t) => [t, score(t)] as const)
    .filter(([, n]) => n >= 0)
    .sort((a, b) => a[1] - b[1])
    .slice(0, limit)
    .map(([t]) => t)
}
