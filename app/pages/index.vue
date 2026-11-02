<script setup lang="ts">
// Home: every running payment once, with its next charge (soonest first, or
// another sort), free trials, paused and ended ones; and what it all costs
// per month / year / day in the main currency.
import {
  Archive,
  ArrowDownUp,
  Check,
  ListFilter,
  X,
  CalendarClock,
  ChevronDown,
  ChevronUp,
  CirclePause,
  MousePointerClick,
  Ban,
  Pencil,
  Plus,
  Search,
  TriangleAlert,
} from 'lucide-vue-next'
import type { SubscriptionView, UpcomingCharge } from '~/composables/api'
import { isFiltered, matches, NO_CATEGORY, NO_FILTER, type HomeFilter } from '~/utils/homeFilter'
import type { Shared } from '~/utils/share'

const router = useRouter()
const { data, error, today } = useOverview()
const toast = useToast()
const adding = ref(false)
// The widget's +: opened with ?add=1.
const route = useRoute()
watch(
  () => route.query.add,
  (v) => {
    if (!v) return
    adding.value = true
    router.replace({ query: {} })
  },
  { immediate: true },
)
// "Share → Hisaab" (Android): opened with the shared text in the query.
const shared = ref<Shared | null>(null)
watch(
  () => route.query.shareText,
  (text) => {
    if (typeof text !== 'string') return
    shared.value = fromShare(text, String(route.query.shareSubject ?? ''))
    adding.value = true
    router.replace({ query: {} })
  },
  { immediate: true },
)

// ---- Filter: only when there are both kinds
const FILTER_KEY = 'hisaab.homeFilter'
const filter = ref<HomeFilter>({ kind: 'all', categories: [] })
try {
  const saved = JSON.parse(localStorage.getItem(FILTER_KEY) ?? 'null')
  if (saved && typeof saved.kind === 'string' && Array.isArray(saved.categories)) filter.value = saved
} catch {}
watch(filter, (v) => {
  try {
    localStorage.setItem(FILTER_KEY, JSON.stringify(v))
  } catch {}
})
const filtering = ref(false)
const filtered = computed(() => isFiltered(filter.value))
/** "Bills · Streaming, Music" for the active filter. */
const filterText = computed(() => {
  const f = filter.value
  const kind = f.kind === 'all' ? '' : f.kind === 'bill' ? 'Bills' : 'Subscriptions'
  const cats = f.categories.map((c) => (c === NO_CATEGORY ? 'No category' : (categoryNames.value.get(c) ?? '…'))).join(', ')
  return [kind, cats, f.hideOver ? 'Without ended' : ''].filter(Boolean).join(' · ')
})
const subs = computed(() => data.value?.subscriptions ?? [])
const shown = (s: SubscriptionView) => matches(filter.value, s, categoryNames.value)
const byId = computed(() => new Map(subs.value.map((s) => [s.id, s])))

// ---- Header total: tap cycles month → year → day
// Monthly budget (Settings): how much of it the running payments use.
const budget = computed(() => {
  const d = data.value
  if (!d?.monthlyBudget) return null
  const monthly = Math.round(d.yearlyTotal / 12)
  const share = monthly / d.monthlyBudget
  const money = (n: number) => formatMoney(n, d.mainCurrency, { compact: true })
  const text =
    monthly > d.monthlyBudget
      ? `${money(monthly - d.monthlyBudget)} over your ${money(d.monthlyBudget)} budget`
      : `${Math.round(share * 100)}% of your ${money(d.monthlyBudget)} budget`
  return { share, text, level: share > 1 ? 'over' : share >= 0.9 ? 'near' : 'ok' }
})
const SPANS = [
  { key: 'month', label: 'month', per: 1 / 12 },
  { key: 'year', label: 'year', per: 1 },
  { key: 'day', label: 'day', per: 1 / 365.2425 },
] as const
const span = ref(0)
const total = computed(() => {
  const d = data.value
  if (!d) return null
  const yearly = subs.value.filter(shown).reduce((sum, s) => sum + (s.yearlyInMain ?? 0), 0)
  return formatMoney(Math.round(yearly * SPANS[span.value]!.per), d.mainCurrency)
})

// ---- One row per payment: its nearest charge from today on (one-time
// charges too); once a charge's day has passed, the next one takes over.
const nextBySub = computed(() => {
  const m = new Map<string, UpcomingCharge>()
  for (const c of data.value?.upcoming ?? []) if (!m.has(c.subscriptionId)) m.set(c.subscriptionId, c)
  return m
})
type Sort = 'next' | 'name' | 'cost' | 'cheap' | 'category'
const SORTS: { key: Sort; label: string }[] = [
  { key: 'next', label: 'Next charge' },
  { key: 'name', label: 'Name' },
  { key: 'cost', label: 'Most expensive' },
  { key: 'cheap', label: 'Cheapest' },
  { key: 'category', label: 'Category' },
]
const SORT_KEY = 'hisaab.homeSort'
const sort = ref<Sort>('next')
try {
  const saved = localStorage.getItem(SORT_KEY) as Sort | null
  if (saved && SORTS.some((x) => x.key === saved)) sort.value = saved
} catch {}
watch(sort, (v) => {
  try {
    localStorage.setItem(SORT_KEY, v)
  } catch {}
})
const categoryNames = ref(new Map<string, string>())
const loadCategories = () =>
  api
    .categories()
    .then((cs) => (categoryNames.value = new Map(cs.map((c) => [c.id, c.name]))))
    .catch(() => {})
watch(dataVersion, loadCategories)
onMounted(loadCategories)
function sortMenu(e: MouseEvent) {
  openMenu(
    e,
    SORTS.map((x) => ({ label: x.label, icon: sort.value === x.key ? Check : undefined, run: () => (sort.value = x.key) })),
  )
}
const rows = computed(() => {
  const list = subs.value
    // Trials with a countdown have their own section above.
    .filter((s) => (s.status === 'active' || (s.status === 'trial' && !trials.value.some((t) => t.s.id === s.id))) && shown(s))
    .map((s) => ({ s, c: nextBySub.value.get(s.id) ?? (s.nextCharge ? { ...s.nextCharge, inMain: s.nextInMain } : null) }))
  const name = (x: (typeof list)[number]) => x.s.name.toLowerCase()
  const cost = (x: (typeof list)[number]) => x.s.yearlyInMain ?? 0
  const due = (x: (typeof list)[number]) => x.c?.dueDate ?? '9999'
  const cat = (x: (typeof list)[number]) => (x.s.categoryId ? (categoryNames.value.get(x.s.categoryId) ?? '~') : '~').toLowerCase()
  const by: Record<Sort, (a: (typeof list)[number], b: (typeof list)[number]) => number> = {
    next: (a, b) => due(a).localeCompare(due(b)) || name(a).localeCompare(name(b)),
    name: (a, b) => name(a).localeCompare(name(b)),
    cost: (a, b) => cost(b) - cost(a) || name(a).localeCompare(name(b)),
    cheap: (a, b) => cost(a) - cost(b) || name(a).localeCompare(name(b)),
    category: (a, b) => cat(a).localeCompare(cat(b)) || due(a).localeCompare(due(b)),
  }
  return list.sort(by[sort.value])
})
// Free trials that turn into paid ones, ending soonest first (cancel in time).
const trials = computed(() =>
  subs.value
    .filter((s) => s.status === 'trial' && s.current?.status === 'trial' && s.current.endDate && s.nextCharge && shown(s))
    .map((s) => ({ s, left: daysBetween(today.value, s.current!.endDate!) }))
    .sort((a, b) => a.left - b.left),
)
const paused = computed(() => subs.value.filter((s) => s.status === 'paused' && shown(s)))
const showPaused = ref(false)
// Ended and archived: still listed, at the bottom and grayed out.
const archived = ref<SubscriptionView[]>([])
const loadArchived = () =>
  api
    .archived(today.value)
    .then((a) => (archived.value = a))
    .catch(() => {})
watch(dataVersion, loadArchived)
onMounted(loadArchived)
const over = computed(() =>
  filter.value.hideOver ? [] : [...subs.value.filter((s) => s.status === 'ended'), ...archived.value].filter(shown),
)
/** "Ended 3 Mar 2025", "Archived". */
const overText = (s: SubscriptionView) =>
  s.archivedAt ? 'Archived' : s.current?.endDate ? `Ended ${shortDay(s.current.endDate, today.value)}` : 'Ended'
const empty = computed(() => data.value && !subs.value.length && !archived.value.length)

// ---- Row actions
const moving = ref<UpcomingCharge | null>(null)
async function moveTo(day: string | null) {
  const c = moving.value
  moving.value = null
  if (c && day && day !== c.dueDate) await moveCharge(c, day)
}
const editing = ref<UpcomingCharge | null>(null)
// Wide window: Detail opens beside the list (?sub=<id>); else its own screen.
const wide = useWide()
const selected = computed(() => (typeof route.query.sub === 'string' ? route.query.sub : null))
const open = (id: string, action?: string) =>
  wide.value
    ? router.replace({ query: { sub: id, ...(action ? { action } : {}) } })
    : router.push({ path: `/sub/${id}`, query: action ? { action } : {} })
const closeDetail = () => router.replace({ query: {} })
// Narrowed while a payment was open: give it its own screen.
watch(wide, (w) => {
  if (!w && selected.value) {
    const { sub, ...query } = route.query
    router.replace({ path: `/sub/${sub}`, query })
  }
})
function menu(e: MouseEvent, c: UpcomingCharge) {
  openMenu(e, [
    { label: 'Move this charge', icon: CalendarClock, run: () => (moving.value = c) },
    { label: 'Change this charge', icon: Pencil, run: () => (editing.value = c) },
    'separator',
    { label: 'Pause', icon: CirclePause, run: () => open(c.subscriptionId, 'pause') },
    { label: 'Change price', icon: Pencil, run: () => open(c.subscriptionId, 'price') },
    { label: 'Cancel', icon: Ban, danger: true, run: () => open(c.subscriptionId, 'cancel') },
  ])
}
</script>

<template>
  <main class="page" :class="{ split: wide }">
    <div class="list-col">
      <header class="topbar">
        <h1 class="brand">Hisaab</h1>
        <SyncDot />
        <span class="grow" />
        <button v-if="wide && !empty" class="icon-btn" aria-label="Add a payment" @click="adding = true"><Plus :size="22" /></button>
        <button v-if="!empty" class="icon-btn filter-btn" aria-label="Filter" title="Filter" @click="filtering = true">
          <ListFilter :size="22" /><span v-if="filtered" class="dot" />
        </button>
        <button v-if="!empty" class="icon-btn" aria-label="Sort" title="Sort" @click="sortMenu"><ArrowDownUp :size="22" /></button>
        <NuxtLink v-if="!empty" to="/search" class="icon-btn" aria-label="Search" title="Search (/)"><Search :size="22" /></NuxtLink>
        <SettingsLink />
      </header>
      <SyncStaleHint />

      <p v-if="error" class="error">{{ error }}</p>

      <template v-if="data && !empty">
        <button class="total" :aria-label="`Total per ${SPANS[span]!.label}, tap to switch`" @click="span = (span + 1) % SPANS.length">
          <span class="amount">{{ total }}</span
          ><span class="muted"> / {{ SPANS[span]!.label }}</span>
        </button>
        <div v-if="budget" class="budget" :class="budget.level">
          <span class="track"><span class="fill" :style="{ width: `${Math.min(1, budget.share) * 100}%` }" /></span>
          <span class="small">{{ budget.text }}</span>
        </div>
        <p v-if="data.excludedCurrencies.length || (data.ratesStale && data.ratesDate)" class="note muted small">
          <TriangleAlert :size="14" />
          <span v-if="data.excludedCurrencies.length">{{ data.excludedCurrencies.join(', ') }} not in totals (no exchange rate)</span>
          <span v-else>Rates from {{ shortDay(data.ratesDate!) }}</span>
        </p>

        <div v-if="filtered" class="chips filters">
          <button class="chip chosen" @click="filtering = true">{{ filterText }}</button>
          <button class="icon-btn" aria-label="Show all" @click="filter = { ...NO_FILTER }"><X :size="18" /></button>
        </div>

        <section v-if="trials.length">
          <h2 class="section-title">Free trials</h2>
          <ul class="cards">
            <li
              v-for="{ s, left } in trials"
              :key="s.id"
              class="tint-card"
              :class="{ selected: wide && s.id === selected }"
              :style="{ '--c': s.color || fallbackColor(s.name) }"
            >
              <button class="sub-row" @click="open(s.id)">
                <SubIcon :icon="s.icon" :color="s.color" :name="s.name" :size="38" />
                <span class="flex stack">
                  <span>{{ s.name }}</span>
                  <span class="small" :class="left <= 2 ? 'soon' : 'muted'">
                    {{ left <= 0 ? 'Ends today' : left === 1 ? 'Ends tomorrow' : `Ends in ${left} days` }}
                  </span>
                </span>
                <span class="muted small">then {{ formatMoney(s.nextCharge!.amountMinor, s.nextCharge!.currency) }}</span>
              </button>
            </li>
          </ul>
        </section>

        <ul v-if="rows.length" class="cards">
          <template v-for="{ s, c } in rows" :key="s.id">
            <ChargeRow
              v-if="c"
              card
              :charge="c"
              :class="{ selected: wide && s.id === selected }"
              :sub="s"
              :main="data.mainCurrency"
              :today="today"
              @open="open(s.id)"
              @move="moving = c"
              @menu="menu($event, c)"
            />
            <li
              v-else
              class="tint-card"
              :class="{ selected: wide && s.id === selected }"
              :style="{ '--c': s.color || fallbackColor(s.name) }"
            >
              <button class="sub-row" @click="open(s.id)">
                <SubIcon :icon="s.icon" :color="s.color" :name="s.name" :size="38" />
                <span class="flex">{{ s.name }}</span>
                <span class="muted small">No charge coming</span>
              </button>
            </li>
          </template>
        </ul>
        <p v-else-if="!trials.length" class="muted small quiet">Nothing running right now.</p>

        <template v-if="paused.length">
          <button class="pane paused-row" :aria-expanded="showPaused" @click="showPaused = !showPaused">
            <CirclePause :size="20" class="muted" />
            <span class="flex">{{ paused.length }} paused</span>
            <component :is="showPaused ? ChevronUp : ChevronDown" :size="20" class="muted" />
          </button>
          <ul v-if="showPaused" class="cards">
            <li
              v-for="s in paused"
              :key="s.id"
              class="tint-card paused"
              :class="{ selected: wide && s.id === selected }"
              :style="{ '--c': s.color || fallbackColor(s.name) }"
            >
              <button class="sub-row" @click="open(s.id)">
                <SubIcon :icon="s.icon" :color="s.color" :name="s.name" :size="38" />
                <span class="flex">{{ s.name }}</span>
                <span v-if="s.current?.endDate" class="muted small">until {{ shortDay(s.current.endDate) }}</span>
              </button>
            </li>
          </ul>
        </template>
        <template v-if="over.length">
          <h2 class="section-title over-title">
            <span class="flex">Ended and archived</span>
            <NuxtLink to="/archive" class="muted small manage" aria-label="Manage ended and archived"
              ><Archive :size="15" /> Manage</NuxtLink
            >
          </h2>
          <ul class="cards">
            <li
              v-for="s in over"
              :key="s.id"
              class="tint-card ended"
              :class="{ selected: wide && s.id === selected }"
              :style="{ '--c': s.color || fallbackColor(s.name) }"
            >
              <button class="sub-row" @click="open(s.id)">
                <SubIcon :icon="s.icon" :color="s.color" :name="s.name" :size="38" />
                <span class="flex">{{ s.name }}</span>
                <span class="muted small">{{ overText(s) }}</span>
              </button>
            </li>
          </ul>
        </template>
      </template>

      <div v-else-if="data" class="empty">
        <CalendarClock :size="40" class="muted" />
        <p>No payments yet</p>
        <p class="muted small">Add your subscriptions and bills; the next charges show up here.</p>
        <button class="btn primary" @click="adding = true"><Plus :size="18" /> Add a payment</button>
      </div>
    </div>
    <aside v-if="wide" class="detail-col pane">
      <SubDetail v-if="selected" :id="selected" embedded @close="closeDetail" />
      <div v-else class="pick muted">
        <MousePointerClick :size="28" />
        <span class="small">Pick a payment to see its history</span>
      </div>
    </aside>

    <button v-if="!empty && !wide" class="fab ink" aria-label="Add a payment" @click="adding = true"><Plus :size="26" /></button>
    <FilterSheet
      v-if="filtering && data"
      v-model="filter"
      :subs="subs.filter((s) => s.status !== 'ended')"
      :names="categoryNames"
      :main="data.mainCurrency"
      @close="filtering = false"
    />
    <QuickAdd v-if="adding" :share="shared" @close="((adding = false), (shared = null))" />
    <ExtraSheet
      v-if="editing?.extraId"
      :subscription-id="editing.subscriptionId"
      :name="byId.get(editing.subscriptionId)?.name ?? ''"
      :currency="editing.currency"
      :extra="extraOf(editing)"
      @close="editing = null"
    />
    <ChargeSheet v-else-if="editing" :charge="editing" :name="byId.get(editing.subscriptionId)?.name ?? ''" @close="editing = null" />
    <DateSheet
      v-if="moving"
      :model-value="moving.dueDate"
      title="Move this charge"
      done-label="Move"
      @update:model-value="moveTo"
      @close="moving = null"
    />
  </main>
</template>

<style scoped>
/* Stays at the top while the list scrolls under it: pulled up over the
   page's top padding and the status bar so nothing shows above it. */
.list-col > .topbar {
  position: sticky;
  top: 0;
  z-index: 40;
  margin: calc(-8px - env(safe-area-inset-top)) -16px 0;
  padding: calc(8px + env(safe-area-inset-top)) 16px 0;
  background: color-mix(in oklch, var(--bg) 80%, transparent);
  -webkit-backdrop-filter: var(--glass-blur);
  backdrop-filter: var(--glass-blur);
}
/* The sync dot sits right after the name (as in Befikr). */
.topbar h1.brand {
  flex: none;
}
.grow {
  flex: 1;
}
.total {
  display: block;
  text-align: left;
  margin: 4px 0 2px;
  padding: 0;
}
.total .amount {
  font-size: 2.2rem;
  font-weight: 600;
  letter-spacing: -0.02em;
  font-variant-numeric: tabular-nums;
}
.note {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 4px 0 0;
}
.filters {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 14px;
}
.filter-btn {
  position: relative;
}
.filter-btn .dot {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent);
}
.list {
  margin: 0;
  padding-top: 4px;
  padding-bottom: 4px;
}
.list > li {
  list-style: none;
}
.over-title {
  display: flex;
  align-items: center;
  margin-top: 24px;
}
.manage {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-weight: 400;
  text-decoration: none;
}
/* Over: still there, grayed out. */
.cards {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 8px 0 0;
  padding: 0;
}
.cards .sub-row {
  padding: 12px 14px;
}
.quiet {
  margin-top: 24px;
  text-align: center;
}
.paused-row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  margin-top: 24px;
  padding: 12px 14px;
  text-align: left;
}
.paused-row + .cards {
  margin-top: 8px;
}
.sub-row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 8px 0;
  text-align: left;
}
.flex {
  flex: 1;
}
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  margin-top: 26vh;
  text-align: center;
}
.empty p {
  margin: 0;
}
.empty .btn {
  margin-top: 14px;
}
.fab {
  position: fixed;
  right: 20px;
  bottom: calc(84px + env(safe-area-inset-bottom));
  z-index: 41;
  width: 56px;
  height: 56px;
  border-radius: 18px;
  display: grid;
  place-items: center;
}
@media (min-width: 900px) {
  .fab {
    bottom: calc(24px + env(safe-area-inset-bottom));
  }
}
/* Wide window: the list, and the open payment beside it (sticky, scrolls on its own). */
.page.split {
  max-width: 1240px;
  display: grid;
  grid-template-columns: minmax(360px, 460px) 1fr;
  gap: 24px;
  padding-left: 112px;
  align-items: start;
}
.detail-col {
  position: sticky;
  top: 8px;
  height: calc(100vh - 16px);
  padding: 0 16px;
  overflow: hidden;
}
.pick {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
}
/* Tinted cards show it with a stronger tint (main.css). */
.selected:not(.tint-card) {
  background: var(--glass-state);
}
.soon {
  color: var(--orange);
}
.stack {
  display: flex;
  flex-direction: column;
  gap: 2px;
  text-align: left;
}
.budget {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 2px 0 14px;
  color: var(--muted);
}
.budget .track {
  height: 6px;
  border-radius: 3px;
  background: var(--line);
  overflow: hidden;
}
.budget .fill {
  display: block;
  height: 100%;
  border-radius: 3px;
  background: var(--green);
}
.budget.near .fill {
  background: var(--orange);
}
.budget.near {
  color: var(--orange);
}
.budget.over .fill {
  background: var(--red);
}
.budget.over {
  color: var(--red);
}
</style>
