// Errors as the user reads them. Commands fail with raw text from Rust
// ("invalid: …", "not found: <id>", "database: …"): known ones get a plain
// sentence, already-friendly ones pass through, the rest become
// "Couldn't <doing>. Try again." The raw text goes to the console.

const KNOWN: [RegExp, string][] = [
  [/name is empty/, 'Add a name first.'],
  [/^not found:/, 'This was deleted, maybe on another device.'],
  [/^database:/, 'Couldn’t save. Try again.'],
]

/** Raw text from Rust, still to be worded. */
const RAW = /^(invalid|not found|database|migration|sync):/

/** `doing`: what failed, for the fallback ("save the subscription"). */
export function friendlyError(e: unknown, doing = 'do that'): string {
  const raw = String(e instanceof Error ? e.message : e).trim()
  console.warn(`${doing}: ${raw}`)
  for (const [re, text] of KNOWN) if (re.test(raw)) return text
  // A full sentence from us.
  const text = raw.replace(/^invalid: /, '')
  if (!RAW.test(text) && /^[A-Z]/.test(text) && /[.!?]$/.test(text)) return text
  return `Couldn’t ${doing}. Try again.`
}

/** A backup's last error (raw from Rust or Android), worded. */
export function friendlyBackupError(raw: string): string {
  if (/permission|denied|os error|no such file|read-only|EACCES|ENOENT|SecurityException/i.test(raw)) {
    return 'Couldn’t write to the backup folder. Choose it again.'
  }
  return friendlyError(raw, 'back up')
}
