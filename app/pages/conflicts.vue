<script setup lang="ts">
// Edits from this device that lost to a concurrent edit on another device
// (both changed the same thing before syncing; the newer one won), from
// Befikr. One card per thing changed, mine on the left, theirs (in use now)
// on the right, differences in red. Tap the side to keep; mine is written
// back as a new edit, so it then wins everywhere. A price changed on both
// devices leaves two overlapping periods: that card keeps one of them.
import { listen } from '@tauri-apps/api/event'
import { Check, Smartphone } from 'lucide-vue-next'
import type { Category, Conflict, ConflictPeriod, PaymentMethod, TimelineLane } from '~/composables/api'

const { show } = useToast()
const list = ref<Conflict[] | null>(null)
const myName = ref('This device')
const error = ref('')
const picked = ref<Record<string, 'mine' | 'theirs'>>({})
// For readable values: period currencies, category and method names.
const lanes = ref<TimelineLane[]>([])
const categories = ref<Category[]>([])
const methods = ref<PaymentMethod[]>([])

async function load() {
  try {
    const [all, l, c, m] = await Promise.all([
      api.conflicts(),
      api.timeline(todayIso()).catch(() => []),
      api.categories().catch(() => []),
      api.paymentMethods().catch(() => []),
    ])
    lanes.value = l
    categories.value = c
    methods.value = m
    list.value = all
  } catch (e) {
    error.value = friendlyError(e, 'load the conflicts')
    list.value = []
  }
}
let unlisten: (() => void) | undefined
onMounted(async () => {
  myName.value =
    (await api.syncStatus().then(
      (s) => s.name,
      () => '',
    )) || 'This device'
  await load()
  unlisten = await listen('sync-changed', load).catch(() => undefined)
})
onUnmounted(() => unlisten?.())

type Line = { label: string; mine: string; theirs: string; differs: boolean }
type Card = {
  key: string
  kind: string
  title: string
  at: number
  from: string
  conflicts: Conflict[]
  lines: Line[]
  mineHint: string
  theirsHint: string
}

const fieldNames: Record<string, string> = {
  name: 'Name',
  icon: 'Icon',
  color: 'Colour',
  category_id: 'Category',
  payment_method_id: 'Paid with',
  url: 'Link',
  notes: 'Notes',
  remind_days_before: 'Remind',
  archived_at: 'Archived',
  deleted_at: 'Status',
  status: 'Status',
  plan_label: 'Plan',
  amount_minor: 'Price',
  currency: 'Currency',
  every_n: 'Every',
  unit: 'Every',
  anchor_date: 'First charge',
  start_date: 'From',
  end_date: 'Until',
  amount_kind: 'Amount',
  kind: 'Change',
  new_due_date: 'Moved to',
  due_date: 'Charge',
  actual_amount_minor: 'Charged',
  settled_amount_minor: 'Bank took',
  value: 'Value',
}
const settingNames: Record<string, string> = {
  main_currency: 'Main currency',
  default_remind_days: 'Remind before a charge',
  reminder_time: 'Reminder time',
  last_currency: 'Last currency',
}

/** A period's currency (for its price). */
const currencyOf = (table: string, rowId: string) =>
  table === 'period' ? (lanes.value.flatMap((l) => l.periods).find((p) => p.id === rowId)?.currency ?? null) : null

const day = (v: unknown) => (typeof v === 'string' && v ? shortDay(v) : '—')

function text(table: string, field: string, v: unknown, rowId: string): string {
  const empty = v === null || v === undefined || v === ''
  if (field === 'deleted_at') return empty ? 'Kept' : 'Deleted'
  if (field === 'archived_at') return empty ? 'No' : 'Yes'
  if (empty) return field === 'end_date' ? 'Open-ended' : '—'
  switch (field) {
    case 'amount_minor':
    case 'actual_amount_minor':
    case 'settled_amount_minor': {
      const c = currencyOf(table, rowId)
      return c && typeof v === 'number' ? formatMoney(v, c) : String(v)
    }
    case 'anchor_date':
    case 'start_date':
    case 'end_date':
    case 'due_date':
    case 'new_due_date':
      return day(v)
    case 'category_id':
      return categories.value.find((c) => c.id === v)?.name ?? '—'
    case 'payment_method_id':
      return methods.value.find((m) => m.id === v)?.name ?? '—'
    case 'remind_days_before':
      return v === 0 ? 'On the day' : `${v} ${v === 1 ? 'day' : 'days'} before`
    case 'icon':
      return String(v).replace(/^(si|lu):/, '')
    case 'value':
      try {
        return String(JSON.parse(String(v)))
      } catch {
        return String(v)
      }
    default:
      return String(v)
  }
}

const periodText = (p: ConflictPeriod) =>
  p.status === 'paused' ? 'Paused' : `${formatMoney(p.amountMinor, p.currency)} ${intervalText(p.everyN, p.unit)}`

const cards = computed<Card[]>(() => {
  const groups = new Map<string, Conflict[]>()
  for (const c of list.value ?? []) {
    const k = `${c.table}:${c.rowId}:${c.field === 'overlap' ? 'overlap' : ''}`
    groups.set(k, [...(groups.get(k) ?? []), c])
  }
  return [...groups.entries()].map(([key, cs]) => {
    const first = cs[0]!
    const from = cs.find((c) => c.wonBy)?.wonBy ?? 'Other device'
    const base = { key, title: first.label, at: Math.max(...cs.map((c) => c.at)), from, conflicts: cs }
    if (first.field === 'overlap') {
      const mine = first.lostValue as ConflictPeriod
      const theirs = first.wonValue as ConflictPeriod
      const line = (label: string, m: string, t: string): Line => ({ label, mine: m, theirs: t, differs: m !== t })
      return {
        ...base,
        kind: 'Price changed twice',
        mineHint: 'Your change',
        theirsHint: 'Their change',
        lines: [line('Price', periodText(mine), periodText(theirs)), line('From', day(mine.start), day(theirs.start))],
      }
    }
    const kind =
      {
        subscription: 'Payment',
        period: 'Period',
        charge_override: 'Charge',
        category: 'Category',
        payment_method: 'Payment method',
        setting: 'Setting',
      }[first.table] ?? 'Change'
    const title = first.table === 'setting' ? (settingNames[first.label] ?? first.label) : first.label
    const lines = cs.map((c) => {
      const mine = text(c.table, c.field, c.lostValue, c.rowId)
      const theirs = text(c.table, c.field, c.wonValue, c.rowId)
      return { label: fieldNames[c.field] ?? c.field, mine, theirs, differs: mine !== theirs }
    })
    return { ...base, kind, title, lines, mineHint: myName.value, theirsHint: 'In use now' }
  })
})

async function pick(card: Card, side: 'mine' | 'theirs') {
  if (picked.value[card.key]) return
  picked.value = { ...picked.value, [card.key]: side }
  // Show the tick for a moment, then the card goes (optimistic).
  await new Promise((r) => setTimeout(r, 350))
  const ids = new Set(card.conflicts.map((c) => c.id))
  list.value = list.value?.filter((c) => !ids.has(c.id)) ?? null
  error.value = ''
  try {
    for (const c of card.conflicts) await api.resolveConflict(c.id, side === 'mine')
    changed()
    show({ message: side === 'mine' ? 'Kept your version' : `Kept ${card.from}’s version` })
  } catch (e) {
    error.value = friendlyError(e, 'keep that version')
    await load()
  } finally {
    const { [card.key]: _, ...rest } = picked.value
    picked.value = rest
  }
}

const ago = (ms: number) => {
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000))
  return s < 60 ? 'just now' : `${span(s * 1000)} ago`
}
</script>

<template>
  <Screen title="Conflicts">
    <p v-if="error" class="error">{{ error }}</p>

    <p v-if="list && !cards.length" class="muted small intro">
      Nothing here. When this device and another change the same thing before they sync, the newer change wins and this device’s version
      shows up here.
    </p>
    <template v-else-if="list">
      <p class="muted small intro">Changed on two devices before they synced. Tap the version to keep.</p>
      <TransitionGroup tag="div" name="list" class="cards">
        <section v-for="card in cards" :key="card.key" class="pane card">
          <div class="head">
            <span class="title">{{ card.title }}</span>
            <span class="muted small">{{ card.kind }} · {{ ago(card.at) }}</span>
          </div>
          <div class="sides">
            <button
              v-for="side in ['mine', 'theirs'] as const"
              :key="side"
              type="button"
              class="side"
              :class="{ chosen: picked[card.key] === side, faded: picked[card.key] && picked[card.key] !== side }"
              :aria-label="side === 'mine' ? `Keep your version (${myName})` : `Keep ${card.from}’s version`"
              @click="pick(card, side)"
            >
              <span class="who">
                <Check v-if="picked[card.key] === side" :size="15" class="tick" />
                <Smartphone v-else :size="15" class="muted" />
                <span class="who-name">{{ side === 'mine' ? 'Yours' : card.from }}</span>
              </span>
              <span class="muted tiny">{{ side === 'mine' ? card.mineHint : card.theirsHint }}</span>
              <span v-for="l in card.lines" :key="l.label" class="line" :class="{ diff: l.differs }">
                <span class="muted tiny">{{ l.label }}</span>
                <span class="val">{{ side === 'mine' ? l.mine : l.theirs }}</span>
              </span>
            </button>
          </div>
        </section>
      </TransitionGroup>
    </template>
  </Screen>
</template>

<style scoped>
.intro {
  margin: 4px 0 12px;
}
.cards {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.card {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px;
}
.head {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 0 2px;
}
.title {
  font-weight: 600;
  overflow-wrap: anywhere;
}
.sides {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}
.side {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
  text-align: left;
  min-width: 0;
  padding: 12px;
  border-radius: 12px;
  box-shadow: inset 0 0 0 1px var(--line);
  transition:
    opacity 0.2s,
    box-shadow 0.2s;
}
.side.chosen {
  box-shadow: inset 0 0 0 2px var(--text);
}
.side.faded {
  opacity: 0.4;
}
.who {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
}
.who-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tick {
  color: var(--text);
}
.tiny {
  font-size: 0.75rem;
}
.line {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 4px 6px;
  margin: 0 -6px;
  border-radius: 6px;
}
.val {
  overflow-wrap: anywhere;
  white-space: pre-wrap;
  font-size: 0.92rem;
}
.diff {
  background: color-mix(in oklch, var(--red) 12%, transparent);
}
.diff .val {
  color: var(--red);
  font-weight: 500;
}
</style>
