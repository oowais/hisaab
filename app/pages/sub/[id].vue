<script setup lang="ts">
// A subscription: what it costs now, its status actions, its history of
// periods and charges, and what it has cost so far.
import { Archive, ArchiveRestore, ExternalLink, MoreVertical, Pencil, Trash2, Undo2 } from 'lucide-vue-next'
import type { Action, Detail, HistoryCharge } from '~/composables/api'

const route = useRoute()
const router = useRouter()
const toast = useToast()
const id = computed(() => String(route.params.id))
const today = todayIso()
const sub = ref<Detail | null>(null)
const error = ref('')

async function load() {
  try {
    sub.value = await api.detail(id.value, today)
    error.value = ''
  } catch (e) {
    error.value = friendlyError(e, 'load it')
  }
}
watch(dataVersion, load)
onMounted(async () => {
  await load()
  // Past charges are valued at the rate of their day: fetch the history's rates once.
  const from = sub.value?.historyFrom
  if (from && navigator.onLine && (await api.ensureRates(from, today).catch(() => false))) await load()
})

const main = ref('EUR')
onMounted(async () => (main.value = (await api.settings().catch(() => null))?.mainCurrency ?? 'EUR'))

// ---- Status actions (only those that make sense now)
const ACTIONS: Record<string, { action: Action | 'archive'; label: string }[]> = {
  trial: [
    { action: 'convert', label: 'Convert to paid' },
    { action: 'cancel', label: 'Cancel' },
  ],
  active: [
    { action: 'pause', label: 'Pause' },
    { action: 'price', label: 'Change price' },
    { action: 'cancel', label: 'Cancel' },
  ],
  paused: [
    { action: 'resume', label: 'Resume' },
    { action: 'cancel', label: 'Cancel' },
  ],
  ended: [
    { action: 'restart', label: 'Restart' },
    { action: 'archive', label: 'Archive' },
  ],
}
const acting = ref<Action | null>(null)
// Opened from a Timeline bar (?period=): that period stands out.
const focusPeriod = computed(() => (typeof route.query.period === 'string' ? route.query.period : null))
const editing = ref(false)
// Opened from Upcoming's menu (?action=pause).
watch(
  () => [sub.value?.status, route.query.action] as const,
  ([status, action]) => {
    if (!status || typeof action !== 'string') return
    if (ACTIONS[status]?.some((a) => a.action === action)) acting.value = action as Action
    router.replace({ query: {} })
  },
)
function act(a: Action | 'archive') {
  if (a === 'archive') return archive(true)
  acting.value = a
}
function done(message: string) {
  acting.value = null
  toast.show({ message })
}

async function archive(on: boolean) {
  try {
    await api.setArchived(id.value, on)
    changed()
    toast.show({
      message: on ? 'Archived' : 'Back from the archive',
      action: on ? { label: 'Undo', run: () => archive(false) } : undefined,
    })
  } catch (e) {
    toast.show({ message: friendlyError(e, 'archive it') })
  }
}
async function remove() {
  const name = sub.value?.name
  try {
    await api.deleteSubscription(id.value)
    changed()
    router.back()
    toast.show({
      message: `Deleted ${name}`,
      action: {
        label: 'Undo',
        run: () =>
          api
            .restoreSubscription(id.value)
            .then(changed)
            .catch(() => {}),
      },
    })
  } catch (e) {
    toast.show({ message: friendlyError(e, 'delete it') })
  }
}
function more(e: MouseEvent) {
  openMenu(e, [
    { label: 'Edit', icon: Pencil, run: () => (editing.value = true) },
    sub.value?.archivedAt
      ? { label: 'Unarchive', icon: ArchiveRestore, run: () => archive(false) }
      : { label: 'Archive', icon: Archive, run: () => archive(true) },
    'separator',
    { label: 'Delete', icon: Trash2, danger: true, run: remove },
  ])
}

// ---- Display
const STATUS = { trial: 'Trial', active: 'Active', paused: 'Paused', ended: 'Ended' }
const money = (minor: number, cur: string) => formatMoney(minor, cur)
const price = computed(() => {
  const p = sub.value?.current
  return p ? `${p.amountKind === 'estimated' ? '~' : ''}${money(p.amountMinor, p.currency)} ${intervalText(p.everyN, p.unit)}` : ''
})
const periods = computed(() =>
  [...(sub.value?.periods ?? [])].reverse().map((p) => ({
    ...p,
    label: [
      p.status === 'trial' ? 'Trial' : p.status === 'paused' ? 'Paused' : p.planLabel,
      p.status === 'paused' ? null : `${money(p.amountMinor, p.currency)} ${intervalText(p.everyN, p.unit)}`,
    ]
      .filter(Boolean)
      .join(' · '),
    range: `${monthYear(p.startDate)} – ${p.endDate ? monthYear(p.endDate) : 'now'}`,
    future: p.startDate > today,
  })),
)

// Mini timeline: each period as a share of the whole span (to today or the last end).
const strip = computed(() => {
  const ps = sub.value?.periods ?? []
  if (!ps.length) return []
  const start = ps[0]!.startDate
  const endOf = (e: string | null) => e ?? (today > start ? today : addDays(start, 30))
  const last = ps.reduce((m, p) => (endOf(p.endDate) > m ? endOf(p.endDate) : m), today)
  const span = Math.max(1, daysBetween(start, last) + 1)
  return ps.map((p) => ({
    id: p.id,
    status: p.status,
    left: (daysBetween(start, p.startDate) / span) * 100,
    width: Math.max(1, ((daysBetween(p.startDate, endOf(p.endDate)) + 1) / span) * 100),
    future: p.startDate > today,
  }))
})

async function undoOverride(c: HistoryCharge) {
  if (!c.overrideId) return
  await api
    .deleteOverride(c.overrideId)
    .then(changed)
    .catch((e) => toast.show({ message: friendlyError(e, 'undo it') }))
}
function chargeMenu(e: MouseEvent, c: HistoryCharge) {
  if (c.overrideId) openMenu(e, [{ label: 'Back to the usual charge', icon: Undo2, run: () => undoOverride(c) }])
}
</script>

<template>
  <Screen :title="sub?.name ?? ''">
    <template #actions>
      <button v-if="sub" class="icon-btn" aria-label="More" @click="more"><MoreVertical :size="22" /></button>
    </template>
    <p v-if="error" class="error">{{ error }}</p>

    <template v-if="sub">
      <section class="pane card">
        <SubIcon :icon="sub.icon" :color="sub.color" :name="sub.name" :size="52" />
        <div class="facts">
          <span class="chip status" :class="sub.status">{{ STATUS[sub.status] }}</span>
          <span v-if="sub.current?.planLabel" class="muted">{{ sub.current.planLabel }}</span>
          <b class="price">{{ price }}</b>
          <span v-if="sub.nextCharge" class="muted small">
            Next {{ relativeDay(sub.nextCharge.dueDate, today) }} ·
            {{ money(sub.nextCharge.amountMinor, sub.nextCharge.currency) }}
            <template v-if="sub.nextCharge.currency !== main && sub.nextInMain != null"> · {{ money(sub.nextInMain, main) }}</template>
          </span>
          <a v-if="sub.url" :href="sub.url" target="_blank" rel="noopener" class="link small"
            ><ExternalLink :size="14" /> {{ sub.url.replace(/^https?:\/\//, '') }}</a
          >
        </div>
      </section>

      <div class="actions">
        <button
          v-for="a in ACTIONS[sub.status]"
          :key="a.action"
          class="btn"
          :class="{ danger: a.action === 'cancel' }"
          @click="act(a.action)"
        >
          {{ a.label }}
        </button>
      </div>

      <div v-if="strip.length" class="strip" aria-hidden="true">
        <span
          v-for="s in strip"
          :key="s.id"
          class="seg"
          :class="[s.status, { future: s.future }]"
          :style="{ left: `${s.left}%`, width: `${s.width}%`, '--c': sub.color || fallbackColor(sub.name) }"
        />
      </div>

      <h2 class="section-title">Periods</h2>
      <ul class="pane rows list">
        <li
          v-for="p in periods"
          :key="p.id"
          :ref="(el) => p.id === focusPeriod && el && (el as HTMLElement).scrollIntoView({ block: 'center' })"
          class="item"
          :class="{ future: p.future, focus: p.id === focusPeriod }"
        >
          <span class="flex">{{ p.label }}</span>
          <span class="muted small">{{ p.range }}</span>
        </li>
      </ul>

      <template v-if="sub.history.length">
        <h2 class="section-title">
          Total spent: {{ money(sub.totalSpent, main) }}
          <span v-if="sub.excludedCurrencies.length" class="muted">(without {{ sub.excludedCurrencies.join(', ') }})</span>
        </h2>
        <ul class="pane rows list">
          <li v-for="c in sub.history" :key="`${c.periodId}-${c.scheduledDate}`" class="item" @contextmenu="chargeMenu($event, c)">
            <span class="flex">
              {{ shortDay(c.dueDate, today) }}
              <span v-if="c.isOverride" class="muted small">· {{ c.dueDate !== c.scheduledDate ? 'moved' : 'changed' }}</span>
            </span>
            <span class="amount">
              <span>{{ money(c.settledAmountMinor ?? c.amountMinor, c.settledCurrency ?? c.currency) }}</span>
              <span v-if="(c.settledCurrency ?? c.currency) !== main && c.inMain != null" class="muted small">{{
                money(c.inMain, main)
              }}</span>
            </span>
            <button v-if="c.overrideId" class="icon-btn" aria-label="Back to the usual charge" @click="undoOverride(c)">
              <Undo2 :size="16" />
            </button>
          </li>
        </ul>
      </template>
    </template>

    <ActionSheet v-if="acting && sub" :action="acting" :sub="sub" @close="acting = null" @done="done" />
    <EditSheet v-if="editing && sub" :sub="sub" @close="editing = false" @done="editing = false" />
  </Screen>
</template>

<style scoped>
.card {
  display: flex;
  gap: 16px;
  align-items: flex-start;
  padding: 16px;
  margin-top: 4px;
}
.facts {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}
.status {
  align-self: flex-start;
  padding: 2px 10px;
  font-size: 0.8rem;
}
.status.trial {
  color: var(--accent);
}
.status.paused {
  color: var(--yellow);
}
.status.ended {
  color: var(--muted);
}
.price {
  font-size: 1.25rem;
}
.link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 14px;
}
.strip {
  position: relative;
  height: 12px;
  margin-top: 22px;
  border-radius: 6px;
  background: var(--glass-state);
  overflow: hidden;
}
.seg {
  position: absolute;
  top: 0;
  bottom: 0;
  background: var(--c);
  border-radius: 6px;
}
.seg.trial {
  background: color-mix(in oklch, var(--c) 35%, transparent);
  outline: 1.5px dashed var(--c);
  outline-offset: -1.5px;
}
.seg.paused {
  background: transparent;
  outline: 1.5px dashed var(--c);
  outline-offset: -1.5px;
}
.seg.future {
  opacity: 0.45;
}
.list {
  margin: 0;
  padding-top: 2px;
  padding-bottom: 2px;
}
.item {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 48px;
  list-style: none;
}
.item.focus {
  margin: 0 -14px;
  padding: 0 14px;
  background: var(--glass-state);
}
.item.future {
  opacity: 0.6;
}
.flex {
  flex: 1;
  min-width: 0;
}
.amount {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  font-variant-numeric: tabular-nums;
}
</style>
