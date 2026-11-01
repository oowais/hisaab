<script setup lang="ts">
// A subscription: what it costs now, its status actions, its history of
// periods and charges (one-time ones too), and what it has cost so far. Its own screen on a
// phone (pages/sub/[id].vue), the right-hand pane beside Upcoming on a wide
// window (`embedded`).
import { Archive, ArchiveRestore, ExternalLink, MoreVertical, Pencil, Plus, Trash2, Undo2 } from 'lucide-vue-next'
import type { Action, Detail, ExtraCharge, HistoryCharge } from '~/composables/api'
import DetailPane from '~/components/DetailPane.vue'
import Screen from '~/components/Screen.vue'

const props = defineProps<{ id: string; embedded?: boolean }>()
const emit = defineEmits<{ close: [] }>()
const route = useRoute()
const router = useRouter()
const toast = useToast()
const id = computed(() => props.id)
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
async function open() {
  sub.value = null
  await load()
  // Past charges are valued at the rate of their day: fetch the history's rates once.
  // (`load` filled it: TS still thinks it's the null from above.)
  const from = (sub.value as Detail | null)?.historyFrom
  if (from && navigator.onLine && (await api.ensureRates(from, today).catch(() => false))) await load()
}
onMounted(open)
watch(id, open)

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
    { action: 'billing', label: 'Change billing day' },
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
// Opened from a reminder's Snooze (?snooze=1): the snooze sheet.
const snoozing = ref(false)
watch(
  () => route.query.snooze,
  (v) => {
    if (!v) return
    snoozing.value = true
    const { snooze: _, ...query } = route.query
    router.replace({ query })
  },
  { immediate: true },
)
// Opened from a Timeline bar (?period=): that period stands out.
const focusPeriod = computed(() => (typeof route.query.period === 'string' ? route.query.period : null))
const editing = ref(false)
// Opened from Upcoming's menu (?action=pause).
watch(
  () => [sub.value?.status, route.query.action] as const,
  ([status, action]) => {
    if (!status || typeof action !== 'string') return
    if (ACTIONS[status]?.some((a) => a.action === action)) acting.value = action as Action
    const { action: _, ...query } = route.query
    router.replace({ query })
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
    if (props.embedded) emit('close')
    else router.back()
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
    { label: 'Add a one-time charge', icon: Plus, run: () => (addingExtra.value = true) },
    sub.value?.archivedAt
      ? { label: 'Unarchive', icon: ArchiveRestore, run: () => archive(false) }
      : { label: 'Archive', icon: Archive, run: () => archive(true) },
    'separator',
    { label: 'Delete', icon: Trash2, danger: true, run: remove },
  ])
}

// ---- Free trial: how long is left, what follows, where to cancel.
const trial = computed(() => {
  const s = sub.value
  const p = s?.current
  if (!s || s.status !== 'trial' || p?.status !== 'trial' || !p.endDate) return null
  const total = Math.max(1, daysBetween(p.startDate, p.endDate) + 1)
  const left = daysBetween(today, p.endDate)
  const after = s.periods.find((x) => x.startDate > p.endDate! && x.status === 'active')
  const then = after
    ? `${money(after.amountMinor, after.currency)} ${intervalText(after.everyN, after.unit)} from ${shortDay(after.anchorDate, today)}`
    : null
  return { end: p.endDate, left, used: Math.min(1, Math.max(0, (total - left - 1) / total)), then }
})
const host = (url: string) =>
  url
    .replace(/^https?:\/\//, '')
    .replace(/^www\./, '')
    .split('/')[0]
async function visit(url: string) {
  await openUrl(url).catch((e) => toast.show({ message: friendlyError(e, 'open the link') }))
}

// ---- Display
const STATUS = { trial: 'Trial', active: 'Active', paused: 'Paused', ended: 'Ended' }
const money = (minor: number, cur: string) => formatMoney(minor, cur)
const price = computed(() => {
  const p = sub.value?.current
  if (!p) return ''
  if (p.status === 'trial' && !p.amountMinor) return 'Free trial'
  return `${money(p.amountMinor, p.currency)} ${intervalText(p.everyN, p.unit)}`
})
const periods = computed(() =>
  [...(sub.value?.periods ?? [])].reverse().map((p) => ({
    ...p,
    label: [
      p.status === 'trial' ? 'Trial' : p.status === 'paused' ? 'Paused' : p.planLabel,
      p.status === 'paused'
        ? null
        : p.status === 'trial' && !p.amountMinor
          ? 'free'
          : `${money(p.amountMinor, p.currency)} ${intervalText(p.everyN, p.unit)}`,
    ]
      .filter(Boolean)
      .join(' · '),
    range:
      p.startDate > today ? `from ${monthYear(p.startDate)}` : `${monthYear(p.startDate)} – ${p.endDate ? monthYear(p.endDate) : 'now'}`,
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

// ---- One charge differed (amount, date, skipped): the charge sheet.
const editingCharge = ref<HistoryCharge | null>(null)

async function undoOverride(c: HistoryCharge) {
  if (!c.overrideId) return
  await api
    .deleteOverride(c.overrideId)
    .then(changed)
    .catch((e) => toast.show({ message: friendlyError(e, 'undo it') }))
}
// ---- Coming: only what differs from the usual (one-time, moved, changed);
// the next charge is in the card at the top (tap it to change it).
const special = computed(() => sub.value?.coming.filter((c) => c.extraId || c.isOverride) ?? [])
const nextCharge = computed<HistoryCharge | null>(() => {
  const s = sub.value
  const n = s?.nextCharge
  if (!s || !n) return null
  const found = [...s.coming, ...s.history].find((c) => !c.extraId && c.periodId === n.periodId && c.scheduledDate === n.scheduledDate)
  return found ?? { ...n, inMain: s.nextInMain, overrideId: null }
})

// ---- One-time charges: add as many as needed; edit or delete one.
const addingExtra = ref(false)
const editingExtra = ref<ExtraCharge | null>(null)
const openCharge = (c: HistoryCharge) => (c.extraId ? (editingExtra.value = extraOf(c)) : (editingCharge.value = c))
async function deleteExtra(c: HistoryCharge) {
  if (!c.extraId) return
  const id = c.extraId
  await api
    .deleteExtra(id)
    .then(changed)
    .then(() =>
      toast.show({ message: 'One-time charge deleted', action: { label: 'Undo', run: () => void api.restoreExtra(id).then(changed) } }),
    )
    .catch((e) => toast.show({ message: friendlyError(e, 'delete it') }))
}

function chargeMenu(e: MouseEvent, c: HistoryCharge) {
  if (c.extraId)
    return openMenu(e, [
      { label: 'Edit this charge', icon: Pencil, run: () => openCharge(c) },
      'separator',
      { label: 'Delete', icon: Trash2, danger: true, run: () => deleteExtra(c) },
    ])
  openMenu(e, [
    { label: 'Edit this charge', icon: Pencil, run: () => (editingCharge.value = c) },
    ...(c.overrideId ? [{ label: 'Back to the usual charge', icon: Undo2, run: () => undoOverride(c) }] : []),
  ])
}
</script>

<template>
  <component :is="embedded ? DetailPane : Screen" :title="sub?.name ?? ''" @close="emit('close')">
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
          <span v-if="sub.myShare != null && sub.current" class="muted small">
            Shared · yours {{ sub.myShare }}%, {{ money(Math.round((sub.current.amountMinor * sub.myShare) / 100), sub.current.currency) }}
          </span>
          <button v-if="sub.nextCharge" type="button" class="muted small next" @click="nextCharge && openCharge(nextCharge)">
            Next {{ dueIn(sub.nextCharge.dueDate, today) }} ·
            {{ money(sub.nextCharge.amountMinor, sub.nextCharge.currency) }}
            <template v-if="sub.nextCharge.currency !== main && sub.nextInMain != null"> · {{ money(sub.nextInMain, main) }}</template>
          </button>
          <button v-if="sub.url && sub.status !== 'trial'" type="button" class="link small" @click="visit(sub.url)">
            <ExternalLink :size="14" /> {{ host(sub.url) }}
          </button>
        </div>
      </section>

      <section v-if="trial" class="pane trial-card" :class="{ soon: trial.left <= 2 }">
        <div class="trial-head">
          <b>{{ trial.left <= 0 ? 'Free trial ends today' : `Free trial · ${trial.left} ${trial.left === 1 ? 'day' : 'days'} left` }}</b>
          <span class="muted small">ends {{ shortDay(trial.end, today) }}</span>
        </div>
        <span class="track"><span class="fill" :style="{ width: `${trial.used * 100}%` }" /></span>
        <span v-if="trial.then" class="muted small">Then {{ trial.then }}, unless you cancel.</span>
        <span v-else class="muted small">Ends by itself; nothing is charged after.</span>
        <div v-if="trial.then && sub.url" class="trial-actions">
          <button class="btn primary" @click="visit(sub.url)"><ExternalLink :size="16" /> Cancel at {{ host(sub.url) }}</button>
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
      <button type="button" class="link small add-extra" @click="addingExtra = true"><Plus :size="14" /> One-time charge</button>

      <template v-if="special.length">
        <h2 class="section-title">Coming</h2>
        <ul class="pane rows list">
          <li v-for="c in special" :key="chargeKey(c)" class="item" @contextmenu="chargeMenu($event, c)">
            <button class="charge" @click="openCharge(c)">
              <span class="flex">
                {{ shortDay(c.dueDate, today) }}
                <span v-if="c.extraId" class="muted small">· {{ c.label ?? 'one-time' }}</span>
                <span v-else-if="c.isOverride" class="muted small">· {{ c.dueDate !== c.scheduledDate ? 'moved' : 'changed' }}</span>
              </span>
              <span class="amount">
                <span>{{ money(c.amountMinor, c.currency) }}</span>
                <span v-if="c.currency !== main && c.inMain != null" class="muted small">{{ money(c.inMain, main) }}</span>
              </span>
            </button>
          </li>
        </ul>
      </template>

      <template v-if="sub.history.length">
        <h2 class="section-title">
          {{ sub.myShare != null ? 'Your part spent' : 'Total spent' }}: {{ money(sub.totalSpent, main) }}
          <span v-if="sub.excludedCurrencies.length" class="muted">(without {{ sub.excludedCurrencies.join(', ') }})</span>
        </h2>
        <ul class="pane rows list">
          <li v-for="c in sub.history" :key="chargeKey(c)" class="item" @contextmenu="chargeMenu($event, c)">
            <button class="charge" @click="openCharge(c)">
              <span class="flex">
                {{ shortDay(c.dueDate, today) }}
                <span v-if="c.extraId" class="muted small">· {{ c.label ?? 'one-time' }}</span>
                <span v-else-if="c.isOverride" class="muted small">· {{ c.dueDate !== c.scheduledDate ? 'moved' : 'changed' }}</span>
              </span>
              <span class="amount">
                <span>{{ money(c.settledAmountMinor ?? c.amountMinor, c.settledCurrency ?? c.currency) }}</span>
                <span v-if="(c.settledCurrency ?? c.currency) !== main && c.inMain != null" class="muted small">{{
                  money(c.inMain, main)
                }}</span>
              </span>
            </button>
            <button v-if="c.overrideId" class="icon-btn" aria-label="Back to the usual charge" @click="undoOverride(c)">
              <Undo2 :size="16" />
            </button>
          </li>
        </ul>
      </template>
    </template>

    <ChargeSheet
      v-if="editingCharge && sub"
      :charge="editingCharge"
      :name="sub.name"
      :override-id="editingCharge.overrideId"
      @close="editingCharge = null"
    />
    <ExtraSheet
      v-if="(addingExtra || editingExtra) && sub"
      :subscription-id="sub.id"
      :name="sub.name"
      :currency="sub.current?.currency ?? main"
      :extra="editingExtra"
      @close="((addingExtra = false), (editingExtra = null))"
    />
    <ActionSheet v-if="acting && sub" :action="acting" :sub="sub" @close="acting = null" @done="done" />
    <SnoozeSheet
      v-if="snoozing && sub"
      :id="sub.id"
      :name="sub.name"
      @close="snoozing = false"
      @done="(m) => ((snoozing = false), done(m))"
    />
    <EditSheet v-if="editing && sub" :sub="sub" @close="editing = false" @done="editing = false" />
  </component>
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
.next {
  padding: 0;
  text-align: left;
  text-decoration: underline dotted;
  text-underline-offset: 3px;
}
.add-extra {
  align-self: flex-start;
  margin: 8px 0 0;
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
.charge {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 48px;
  text-align: left;
  color: inherit;
}
.amount {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  font-variant-numeric: tabular-nums;
}
.trial-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px;
  margin-top: 12px;
}
.trial-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
}
.track {
  height: 6px;
  border-radius: 3px;
  background: var(--line);
  overflow: hidden;
}
.fill {
  display: block;
  height: 100%;
  border-radius: 3px;
  background: var(--text);
}
.trial-card.soon b {
  color: var(--orange);
}
.trial-card.soon .fill {
  background: var(--orange);
}
.trial-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 4px;
}
</style>
