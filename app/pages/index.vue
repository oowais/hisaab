<script setup lang="ts">
// Upcoming (home): what's charged next, grouped by when, and what it all
// costs per month / year / day in the main currency.
import {
  Archive,
  CalendarClock,
  CalendarDays,
  ChevronDown,
  ChevronUp,
  CirclePause,
  MousePointerClick,
  Ban,
  Pencil,
  Plus,
  Search,
  Settings,
  TriangleAlert,
} from 'lucide-vue-next'
import type { SubscriptionView, UpcomingCharge } from '~/composables/api'
import type { Shared } from '~/utils/share'

const update = useUpdate()
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
type Filter = 'all' | 'subscription' | 'bill'
const filter = ref<Filter>('all')
const subs = computed(() => data.value?.subscriptions ?? [])
const bothKinds = computed(() => subs.value.some((s) => s.kind === 'bill') && subs.value.some((s) => s.kind === 'subscription'))
const shown = (s: SubscriptionView) => filter.value === 'all' || s.kind === filter.value
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

// ---- Groups: Today · This week · Later this month · Next month
const groups = computed(() => {
  const d = data.value
  if (!d) return []
  const t = today.value
  const dow = (parseDay(t).getDay() + 6) % 7 // Monday 0
  const weekEnd = addDays(t, 6 - dow)
  const monthKey = t.slice(0, 7)
  const out: { id: string; title: string; rows: UpcomingCharge[] }[] = [
    { id: 'today', title: 'Today', rows: [] },
    { id: 'week', title: 'This week', rows: [] },
    { id: 'month', title: 'Later this month', rows: [] },
    { id: 'next', title: 'Next month', rows: [] },
  ]
  for (const c of d.upcoming) {
    const s = byId.value.get(c.subscriptionId)
    if (!s || !shown(s)) continue
    const g = c.dueDate === t ? 0 : c.dueDate <= weekEnd ? 1 : c.dueDate.slice(0, 7) === monthKey ? 2 : 3
    out[g]!.rows.push(c)
  }
  return out.filter((g) => g.rows.length)
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
const endedCount = computed(() => subs.value.filter((s) => s.status === 'ended').length)
const empty = computed(() => data.value && !subs.value.some((s) => s.status !== 'ended'))

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
        <h1>Hisaab</h1>
        <SyncDot />
        <button v-if="wide && !empty" class="icon-btn" aria-label="Add a payment" @click="adding = true"><Plus :size="22" /></button>
        <NuxtLink v-if="!empty" to="/search" class="icon-btn" aria-label="Search" title="Search (/)"><Search :size="22" /></NuxtLink>
        <NuxtLink to="/calendar" class="icon-btn cal" aria-label="Calendar"><CalendarDays :size="22" /></NuxtLink>
        <NuxtLink to="/settings" class="icon-btn gear" :aria-label="update.available.value ? 'Settings (update available)' : 'Settings'">
          <Settings :size="22" /><span v-if="update.available.value" class="update-dot" />
        </NuxtLink>
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

        <div v-if="bothKinds" class="chips filters" role="radiogroup" aria-label="Show">
          <button
            v-for="f in ['all', 'subscription', 'bill'] as const"
            :key="f"
            class="chip"
            role="radio"
            :aria-checked="filter === f"
            :class="{ chosen: filter === f }"
            @click="filter = f"
          >
            {{ { all: 'All', subscription: 'Subscriptions', bill: 'Bills' }[f] }}
          </button>
        </div>

        <section v-if="trials.length">
          <h2 class="section-title">Free trials</h2>
          <ul class="pane rows list">
            <li v-for="{ s, left } in trials" :key="s.id" :class="{ selected: wide && s.id === selected }">
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

        <section v-for="g in groups" :key="g.id">
          <h2 class="section-title">{{ g.title }}</h2>
          <ul class="pane rows list">
            <ChargeRow
              v-for="c in g.rows"
              :key="chargeKey(c)"
              :charge="c"
              :class="{ selected: wide && c.subscriptionId === selected }"
              :sub="byId.get(c.subscriptionId)!"
              :main="data.mainCurrency"
              :today="today"
              @open="open(c.subscriptionId)"
              @move="moving = c"
              @menu="menu($event, c)"
            />
          </ul>
        </section>
        <p v-if="!groups.length" class="muted small quiet">Nothing due until the end of next month.</p>

        <template v-if="paused.length">
          <button class="pane paused-row" :aria-expanded="showPaused" @click="showPaused = !showPaused">
            <CirclePause :size="20" class="muted" />
            <span class="flex">{{ paused.length }} paused</span>
            <component :is="showPaused ? ChevronUp : ChevronDown" :size="20" class="muted" />
          </button>
          <ul v-if="showPaused" class="pane rows list">
            <li v-for="s in paused" :key="s.id">
              <button class="sub-row" @click="open(s.id)">
                <SubIcon :icon="s.icon" :color="s.color" :name="s.name" :size="38" />
                <span class="flex">{{ s.name }}</span>
                <span v-if="s.current?.endDate" class="muted small">until {{ shortDay(s.current.endDate) }}</span>
              </button>
            </li>
          </ul>
        </template>
        <NuxtLink v-if="endedCount" to="/archive" class="archive-link muted small">
          <Archive :size="15" /> {{ endedCount }} ended
        </NuxtLink>
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
.gear {
  position: relative;
}
.update-dot {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent);
  box-shadow: 0 0 0 2px var(--bg);
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
  gap: 8px;
  margin-top: 14px;
}
.list {
  margin: 0;
  padding-top: 4px;
  padding-bottom: 4px;
}
.list > li {
  list-style: none;
}
.archive-link {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  margin: 20px auto 0;
  color: var(--muted);
  text-decoration: none;
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
.paused-row + .list {
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
.selected {
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
