<script setup lang="ts">
// Upcoming (home): what's charged next, grouped by when, and what it all
// costs per month / year / day in the main currency.
import { CalendarClock, ChevronDown, ChevronUp, CirclePause, Ban, Pencil, Plus, Settings, TriangleAlert } from 'lucide-vue-next'
import type { SubscriptionView, UpcomingCharge } from '~/composables/api'

const update = useUpdate()
const router = useRouter()
const { data, error, today } = useOverview()
const toast = useToast()
const adding = ref(false)

// ---- Filter: only when there are both kinds
type Filter = 'all' | 'subscription' | 'bill'
const filter = ref<Filter>('all')
const subs = computed(() => data.value?.subscriptions ?? [])
const bothKinds = computed(() => subs.value.some((s) => s.kind === 'bill') && subs.value.some((s) => s.kind === 'subscription'))
const shown = (s: SubscriptionView) => filter.value === 'all' || s.kind === filter.value
const byId = computed(() => new Map(subs.value.map((s) => [s.id, s])))

// ---- Header total: tap cycles month → year → day
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
const paused = computed(() => subs.value.filter((s) => s.status === 'paused' && shown(s)))
const showPaused = ref(false)
const empty = computed(() => data.value && !subs.value.some((s) => s.status !== 'ended'))

// ---- Row actions
const moving = ref<UpcomingCharge | null>(null)
async function moveTo(day: string | null) {
  const c = moving.value
  moving.value = null
  if (!c || !day || day === c.dueDate) return
  try {
    await api.setOverride({ periodId: c.periodId, dueDate: c.scheduledDate, kind: 'moved', newDueDate: day })
    changed()
    toast.show({ message: `Moved to ${relativeDay(day)}` })
  } catch (e) {
    toast.show({ message: friendlyError(e, 'move the charge') })
  }
}
const open = (id: string, action?: string) => router.push({ path: `/sub/${id}`, query: action ? { action } : {} })
function menu(e: MouseEvent, c: UpcomingCharge) {
  openMenu(e, [
    { label: 'Move this charge', icon: CalendarClock, run: () => (moving.value = c) },
    'separator',
    { label: 'Pause', icon: CirclePause, run: () => open(c.subscriptionId, 'pause') },
    { label: 'Change price', icon: Pencil, run: () => open(c.subscriptionId, 'price') },
    { label: 'Cancel', icon: Ban, danger: true, run: () => open(c.subscriptionId, 'cancel') },
  ])
}
</script>

<template>
  <main class="page">
    <header class="topbar">
      <h1>Hisaab</h1>
      <SyncDot />
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

      <section v-for="g in groups" :key="g.id">
        <h2 class="section-title">{{ g.title }}</h2>
        <ul class="pane rows list">
          <ChargeRow
            v-for="c in g.rows"
            :key="`${c.periodId}-${c.scheduledDate}`"
            :charge="c"
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
    </template>

    <div v-else-if="data" class="empty">
      <CalendarClock :size="40" class="muted" />
      <p>No payments yet</p>
      <p class="muted small">Add your subscriptions and bills; the next charges show up here.</p>
      <button class="btn primary" @click="adding = true"><Plus :size="18" /> Add a payment</button>
    </div>

    <button v-if="!empty" class="fab ink" aria-label="Add a payment" @click="adding = true"><Plus :size="26" /></button>
    <QuickAdd v-if="adding" @close="adding = false" />
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
</style>
