// Icons for subscriptions, stored as text in `subscription.icon`:
// "si:<slug>" = a bundled Simple Icons brand glyph (CC0, offline), "lu:<Name>"
// = a Lucide icon for generic bills; anything else / none = the name's first
// letter. Only the icons imported here end up in the bundle.
import type { Component } from 'vue'
import {
  siAppletv,
  siApplemusic,
  siApple,
  siAudible,
  siBackblaze,
  siBandcamp,
  siBitwarden,
  siClaude,
  siCloudflare,
  siCrunchyroll,
  siCursor,
  siDazn,
  siDeezer,
  siDigitalocean,
  siDropbox,
  siDuolingo,
  siExpressvpn,
  siFigma,
  siGithub,
  siGoogle,
  siHetzner,
  siIcloud,
  siJellyfin,
  siJetbrains,
  siKagi,
  siLinear,
  siMax,
  siMistralai,
  siMullvad,
  siNetflix,
  siNordvpn,
  siNotion,
  siO2,
  siObsidian,
  siParamountplus,
  siPatreon,
  siPerplexity,
  siPlaystation,
  siPlex,
  siPocketcasts,
  siProton,
  siProtondrive,
  siProtonmail,
  siProtonvpn,
  siRtl,
  siSky,
  siSoundcloud,
  siSpotify,
  siSteam,
  siStrava,
  siTailscale,
  siTidal,
  siTinder,
  siTwitch,
  siVodafone,
  siYoutube,
  si1password,
} from 'simple-icons'
import {
  Baby,
  BookOpen,
  Briefcase,
  Bus,
  Car,
  Cloud,
  Code,
  CreditCard,
  Droplet,
  Dumbbell,
  Film,
  Flame,
  Gamepad2,
  Gift,
  GraduationCap,
  HardDrive,
  Heart,
  HeartPulse,
  House,
  Landmark,
  Mail,
  Music,
  Newspaper,
  Package,
  PawPrint,
  PiggyBank,
  Plane,
  Radio,
  Server,
  Shield,
  ShoppingCart,
  Smartphone,
  Sofa,
  Sparkles,
  Bot,
  TrainFront,
  Trash2,
  Tv,
  Wifi,
  Wrench,
  Zap,
} from 'lucide-vue-next'

/** Bundled brand glyphs (for the picker: name and brand colour too). */
export const BRANDS: { slug: string; title: string; hex: string; path: string }[] = [
  siAppletv,
  siApplemusic,
  siApple,
  siAudible,
  siBackblaze,
  siBandcamp,
  siBitwarden,
  siClaude,
  siCloudflare,
  siCrunchyroll,
  siCursor,
  siDazn,
  siDeezer,
  siDigitalocean,
  siDropbox,
  siDuolingo,
  siExpressvpn,
  siFigma,
  siGithub,
  siGoogle,
  siHetzner,
  siIcloud,
  siJellyfin,
  siJetbrains,
  siKagi,
  siLinear,
  siMax,
  siMistralai,
  siMullvad,
  siNetflix,
  siNordvpn,
  siNotion,
  siO2,
  siObsidian,
  siParamountplus,
  siPatreon,
  siPerplexity,
  siPlaystation,
  siPlex,
  siPocketcasts,
  siProton,
  siProtondrive,
  siProtonmail,
  siProtonvpn,
  siRtl,
  siSky,
  siSoundcloud,
  siSpotify,
  siSteam,
  siStrava,
  siTailscale,
  siTidal,
  siTinder,
  siTwitch,
  siVodafone,
  siYoutube,
  si1password,
]
  .map((i) => ({ slug: i.slug, title: i.title, hex: `#${i.hex}`, path: i.path }))
  .sort((a, b) => a.title.localeCompare(b.title))

export const BRAND_PATHS: Record<string, string> = Object.fromEntries(BRANDS.map((b) => [b.slug, b.path]))

export const GENERIC_ICONS: Record<string, Component> = {
  Baby,
  BookOpen,
  Briefcase,
  Bus,
  Car,
  Cloud,
  Code,
  CreditCard,
  Droplet,
  Dumbbell,
  Film,
  Flame,
  Gamepad2,
  Gift,
  GraduationCap,
  HardDrive,
  Heart,
  HeartPulse,
  House,
  Landmark,
  Mail,
  Music,
  Newspaper,
  Package,
  PawPrint,
  PiggyBank,
  Plane,
  Radio,
  Server,
  Shield,
  ShoppingCart,
  Smartphone,
  Sofa,
  Sparkles,
  Bot,
  TrainFront,
  Trash2,
  Tv,
  Wifi,
  Wrench,
  Zap,
}

export type IconSpec = { kind: 'brand'; path: string } | { kind: 'generic'; component: Component } | { kind: 'letter'; letter: string }

export function iconSpec(icon: string | null | undefined, name: string): IconSpec {
  if (icon?.startsWith('si:') && BRAND_PATHS[icon.slice(3)]) return { kind: 'brand', path: BRAND_PATHS[icon.slice(3)]! }
  if (icon?.startsWith('lu:') && GENERIC_ICONS[icon.slice(3)]) return { kind: 'generic', component: GENERIC_ICONS[icon.slice(3)]! }
  return { kind: 'letter', letter: (name.trim()[0] ?? '?').toUpperCase() }
}

/** White or near-black glyph on a `hex` tile, whichever reads better. */
export function glyphOn(hex: string): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex)
  if (!m) return '#fff'
  const n = parseInt(m[1]!, 16)
  const lin = (c: number) => (c / 255 <= 0.03928 ? c / 255 / 12.92 : ((c / 255 + 0.055) / 1.055) ** 2.4)
  const l = 0.2126 * lin(n >> 16) + 0.7152 * lin((n >> 8) & 255) + 0.0722 * lin(n & 255)
  return l > 0.45 ? '#111' : '#fff'
}

/** Tile colours for subscriptions without one (picked by name), and the picker's swatches. */
export const PALETTE = [
  '#5b6ee1',
  '#e0644f',
  '#3f9a6e',
  '#c48a2c',
  '#9a5bc4',
  '#2f8fb0',
  '#c4507d',
  '#6b7a8f',
  '#111111',
  '#e50914',
  '#1ed760',
  '#ffb319',
]
export function fallbackColor(name: string): string {
  let h = 7
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0
  // Only the first 8: adding swatches must not recolour existing tiles.
  return PALETTE[h % 8]!
}

/** A colour for a new payment that has no template: one of the first 8, at random. */
export function randomColor(): string {
  return PALETTE[Math.floor(Math.random() * 8)]!
}
