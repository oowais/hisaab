<script setup lang="ts">
// Stats: spend per month (stacked by category, from the periods so past
// prices stay right; this month and next are a forecast), what the running
// payments cost split by category, payment method and currency, and what
// pausing and cancelling saved. Hand-built SVG, like the Timeline.
import { ChartColumn, PiggyBank } from 'lucide-vue-next'
import type { Category, PaymentMethod, Slice, Stats } from '~/composables/api'

const today = todayIso()
const range = ref<12 | 0>(12)
const st = ref<Stats | null>(null)
const categories = ref<Category[]>([])
const methods = ref<PaymentMethod[]>([])
const error = ref('')
const picked = ref<string | null>(null)

async function load() {
  try {
    st.value = await api.stats(today, range.value)
    error.value = ''
  } catch (e) {
    error.value = friendlyError(e, 'load the stats')
  }
}
onMounted(async () => {
  ;[categories.value, methods.value] = await Promise.all([api.categories().catch(() => []), api.paymentMethods().catch(() => [])])
  await load()
  // Past charges are valued at their day's rate: fetch the history's rates once.
  const from = st.value?.from
  if (from && navigator.onLine && (await api.ensureRates(from, today).catch(() => false))) await load()
})
watch(dataVersion, load)
watch(range, () => {
  picked.value = null
  load()
})

const main = computed(() => st.value?.mainCurrency ?? 'EUR')
const money = (minor: number) => formatMoney(minor, main.value, { compact: true })
const empty = computed(() => st.value && !st.value.months.some((m) => m.total) && !st.value.yearly)

// ---- Colours: a category's own, else a palette swatch by its order.
const catById = computed(() => new Map(categories.value.map((c) => [c.id, c])))
function colorOf(key: string | null): string {
  if (!key) return 'var(--muted)'
  const c = catById.value.get(key)
  if (c?.color) return c.color
  const i = categories.value.findIndex((x) => x.id === key)
  return PALETTE[(i < 0 ? 0 : i) % 8]!
}
const catName = (key: string | null) => (key ? (catById.value.get(key)?.name ?? 'Category') : 'No category')
const methodName = (key: string | null) => (key ? (methods.value.find((m) => m.id === key)?.name ?? 'Method') : 'Not set')
const currencyName = (key: string | null) => (key ? (new Intl.DisplayNames(undefined, { type: 'currency' }).of(key) ?? key) : '—')

// ---- Monthly chart
const H = 140
const BAR = 18
const GAP = 8
const chart = computed(() => {
  const ms = st.value?.months ?? []
  const max = Math.max(1, ...ms.map((m) => m.total))
  return {
    width: ms.length * (BAR + GAP) - GAP,
    bars: ms.map((m, i) => {
      let y = H
      const segs = m.byCategory.map((s) => {
        const h = (s.amount / max) * H
        y -= h
        return { key: s.key, y, h }
      })
      return { ...m, x: i * (BAR + GAP), segs, label: monthInitial(m.month), year: m.month.endsWith('-01') ? m.month.slice(0, 4) : '' }
    }),
  }
})
function monthInitial(month: string) {
  return new Date(`${month}-01T00:00`).toLocaleDateString(undefined, { month: 'narrow' })
}
const monthName = (month: string) => new Date(`${month}-01T00:00`).toLocaleDateString(undefined, { month: 'long', year: 'numeric' })
const pickedMonth = computed(() => st.value?.months.find((m) => m.month === picked.value) ?? null)
// The newest month that's done: shown when nothing is picked.
const shownMonth = computed(() => pickedMonth.value ?? [...(st.value?.months ?? [])].reverse().find((m) => !m.forecast) ?? null)
const scroller = ref<HTMLElement>()
watch(chart, () => nextTick(() => scroller.value && (scroller.value.scrollLeft = scroller.value.scrollWidth)))

// ---- Splits of the yearly cost
type Split = { title: string; rows: (Slice & { name: string; color: string; share: number })[] }
const splits = computed<Split[]>(() => {
  const s = st.value
  if (!s?.yearly) return []
  const rows = (slices: Slice[], name: (k: string | null) => string, color: (k: string | null) => string) =>
    slices.map((x) => ({ ...x, name: name(x.key), color: color(x.key), share: x.amount / s.yearly }))
  const out: Split[] = [{ title: 'By category', rows: rows(s.byCategory, catName, colorOf) }]
  if (s.byMethod.some((x) => x.key)) out.push({ title: 'By payment method', rows: rows(s.byMethod, methodName, () => 'var(--text)') })
  if (s.byCurrency.length > 1) out.push({ title: 'By currency', rows: rows(s.byCurrency, currencyName, () => 'var(--text)') })
  return out
})
</script>

<template>
  <main class="page">
    <header class="topbar">
      <h1>Stats</h1>
    </header>
    <p v-if="error" class="error">{{ error }}</p>

    <template v-if="st && !empty">
      <div class="totals">
        <div class="total">
          <span class="amount">{{ money(Math.round(st.yearly / 12)) }}</span>
          <span class="muted small">a month now</span>
        </div>
        <div class="total">
          <span class="amount">{{ formatMoney(Math.round(st.yearly / 365), main) }}</span>
          <span class="muted small">a day</span>
        </div>
        <div class="total">
          <span class="amount">{{ money(st.yearly) }}</span>
          <span class="muted small">a year</span>
        </div>
      </div>
      <p v-if="st.excludedCurrencies.length" class="muted small note">{{ st.excludedCurrencies.join(', ') }} left out (no exchange rate)</p>

      <div class="head-row">
        <h2 class="section-title">Spent per month</h2>
        <div class="chips" role="radiogroup" aria-label="Range">
          <button
            v-for="r in [12, 0] as const"
            :key="r"
            class="chip"
            role="radio"
            :aria-checked="range === r"
            :class="{ chosen: range === r }"
            @click="range = r"
          >
            {{ r ? '12 months' : 'All' }}
          </button>
        </div>
      </div>
      <section class="pane chart-pane">
        <div v-if="shownMonth" class="month-line">
          <b>{{ money(shownMonth.total) }}</b>
          <span class="muted small">{{ monthName(shownMonth.month) }}{{ shownMonth.forecast ? ' · expected' : '' }}</span>
        </div>
        <div ref="scroller" class="scroller">
          <svg :width="chart.width" :height="H + 34" :viewBox="`0 0 ${chart.width} ${H + 34}`" role="img" aria-label="Spend per month">
            <g
              v-for="b in chart.bars"
              :key="b.month"
              class="bar"
              :class="{ forecast: b.forecast, dim: picked && picked !== b.month }"
              @click="picked = picked === b.month ? null : b.month"
            >
              <rect :x="b.x - GAP / 2" y="0" :width="BAR + GAP" :height="H + 34" fill="transparent" />
              <rect
                v-for="s in b.segs"
                :key="s.key ?? 'none'"
                :x="b.x"
                :y="s.y"
                :width="BAR"
                :height="Math.max(0, s.h - 1)"
                rx="3"
                :fill="colorOf(s.key)"
              />
              <text :x="b.x + BAR / 2" :y="H + 14" text-anchor="middle" class="axis">{{ b.label }}</text>
              <text v-if="b.year" :x="b.x" :y="H + 30" class="axis year">{{ b.year }}</text>
            </g>
          </svg>
        </div>
        <ul v-if="shownMonth?.byCategory.length" class="legend">
          <li v-for="s in shownMonth.byCategory" :key="s.key ?? 'none'">
            <span class="swatch" :style="{ background: colorOf(s.key) }" />
            <span class="flex">{{ catName(s.key) }}</span>
            <span class="num">{{ money(s.amount) }}</span>
          </li>
        </ul>
      </section>

      <section v-if="st.saved" class="pane saved">
        <PiggyBank :size="22" class="muted" />
        <span class="flex stack">
          <b>{{ money(st.saved) }} saved</b>
          <span class="muted small">by pausing and cancelling, at the prices you had</span>
        </span>
      </section>

      <template v-for="sp in splits" :key="sp.title">
        <h2 class="section-title">{{ sp.title }} <span class="muted small">· a year</span></h2>
        <ul class="pane rows split">
          <li v-for="r in sp.rows" :key="r.key ?? 'none'">
            <span class="split-row">
              <span class="flex">{{ r.name }}</span>
              <span class="num">{{ money(r.amount) }}</span>
              <span class="muted small pct">{{ Math.round(r.share * 100) }}%</span>
            </span>
            <span class="track"><span class="fill" :style="{ width: `${r.share * 100}%`, background: r.color }" /></span>
          </li>
        </ul>
      </template>
    </template>

    <div v-else-if="st" class="empty">
      <ChartColumn :size="40" class="muted" />
      <p>Nothing to count yet</p>
      <p class="muted small">Add payments; what they cost shows up here.</p>
    </div>
  </main>
</template>

<style scoped>
.totals {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  margin: 4px 0 8px;
}
.total {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.total .amount {
  font-size: 1.2rem;
  font-weight: 650;
  font-variant-numeric: tabular-nums;
}
.note {
  margin: 0 0 8px;
}
.head-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin: 20px 0 8px;
}
.head-row .section-title {
  margin: 0;
}
.chips {
  display: flex;
  gap: 6px;
}
.chart-pane {
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.month-line {
  display: flex;
  align-items: baseline;
  gap: 8px;
}
.month-line b {
  font-size: 1.15rem;
  font-variant-numeric: tabular-nums;
}
.scroller {
  overflow-x: auto;
  scrollbar-width: none;
}
svg {
  display: block;
  margin-left: auto;
}
.bar {
  cursor: pointer;
  transition: opacity 0.15s;
}
.bar.forecast rect:not([fill='transparent']) {
  opacity: 0.4;
}
.bar.dim {
  opacity: 0.35;
}
.axis {
  font-size: 10px;
  fill: var(--muted);
}
.legend {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.legend li {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.92rem;
}
.swatch {
  width: 10px;
  height: 10px;
  border-radius: 3px;
  flex: none;
}
.num {
  font-variant-numeric: tabular-nums;
}
.flex {
  flex: 1;
  min-width: 0;
}
.stack {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.saved {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px;
  margin-top: 14px;
}
.split {
  margin: 0;
  list-style: none;
}
.split li {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 0;
}
.split-row {
  display: flex;
  align-items: baseline;
  gap: 10px;
}
.pct {
  width: 3ch;
  text-align: right;
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
}
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  margin-top: 30vh;
  text-align: center;
}
.empty p {
  margin: 0;
}
</style>
