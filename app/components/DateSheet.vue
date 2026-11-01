<script setup lang="ts">
// Date picker: a glass sheet with quick picks and a month calendar (tap the
// month for a month grid, its year for a year grid). Emits a
// 'YYYY-MM-DD' day. Built from Befikr's DateTimeSheet, without the time
// wheels (charges are due on days). Never the native pickers.
import { Check, ChevronDown, ChevronLeft, ChevronRight, X } from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    modelValue: string | null
    title?: string
    /** Extra quick picks, e.g. { label: 'Next charge', day: '2026-03-05' }. */
    picks?: { label: string; day: string }[]
    min?: string | null
    allowClear?: boolean
    clearLabel?: string
    doneLabel?: string
  }>(),
  { title: 'Date', picks: () => [], min: null, allowClear: false, clearLabel: 'None', doneLabel: 'Done' },
)
const emit = defineEmits<{ 'update:modelValue': [day: string | null]; close: [] }>()
useBackClose(() => emit('close'))

const today = todayIso()
const day = ref(props.modelValue ?? (props.min && props.min > today ? props.min : today))
const month = ref(day.value.slice(0, 7) + '-01')

const quick = computed(() => {
  const base = [
    { label: 'Today', day: today },
    { label: 'Tomorrow', day: addDays(today, 1) },
  ]
  const seen = new Set<string>()
  return [...props.picks, ...base].filter((q) => !seen.has(q.label) && !!seen.add(q.label))
})
function pick(d: string) {
  day.value = d
  month.value = d.slice(0, 7) + '-01'
  view.value = 'days'
}

// ---- Calendar (Monday first)
const monthLabel = computed(() => parseDay(month.value).toLocaleDateString(undefined, { month: 'long', year: 'numeric' }))
const weekdays = Array.from({ length: 7 }, (_, i) => new Date(2024, 0, 1 + i).toLocaleDateString(undefined, { weekday: 'narrow' }))
const cells = computed(() => {
  const first = parseDay(month.value)
  const lead = (first.getDay() + 6) % 7
  const start = addDays(month.value, -lead)
  return Array.from({ length: 42 }, (_, i) => {
    const d = addDays(start, i)
    return { d, n: parseDay(d).getDate(), out: d.slice(0, 7) !== month.value.slice(0, 7), past: props.min != null && d < props.min }
  })
})
function shiftMonth(n: number) {
  const d = parseDay(month.value)
  d.setMonth(d.getMonth() + n)
  month.value = isoDay(d)
}

// ---- Jump: tap the month's name for the months of its year, the year
// for a grid of years; picking one goes back down to the days.
const view = ref<'days' | 'months' | 'years'>('days')
const year = computed(() => Number(month.value.slice(0, 4)))
const minMonth = computed(() => props.min?.slice(0, 7) ?? null)
const months = computed(() =>
  Array.from({ length: 12 }, (_, i) => {
    const key = `${year.value}-${String(i + 1).padStart(2, '0')}`
    return {
      key,
      label: new Date(year.value, i, 1).toLocaleDateString(undefined, { month: 'short' }),
      past: minMonth.value != null && key < minMonth.value,
    }
  }),
)
const yearStart = ref(year.value - 4)
const years = computed(() =>
  Array.from({ length: 12 }, (_, i) => {
    const y = yearStart.value + i
    return { y, past: props.min != null && y < Number(props.min.slice(0, 4)) }
  }),
)
function shiftYear(n: number) {
  month.value = `${year.value + n}${month.value.slice(4)}`
}
function pickMonth(key: string) {
  month.value = `${key}-01`
  view.value = 'days'
}
function pickYear(y: number) {
  month.value = `${y}${month.value.slice(4)}`
  view.value = 'months'
}
function openYears() {
  yearStart.value = year.value - 4
  view.value = 'years'
}

const tooEarly = computed(() => props.min != null && day.value < props.min)
function done() {
  if (tooEarly.value) return
  emit('update:modelValue', day.value)
  emit('close')
}
function clear() {
  emit('update:modelValue', null)
  emit('close')
}
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <div class="sheet popover" role="dialog" :aria-label="title">
      <header class="head">
        <b>{{ title }}</b>
        <button class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>

      <div class="chips">
        <button
          v-for="q in quick"
          :key="q.label"
          class="chip"
          :class="{ chosen: q.day === day }"
          :disabled="min != null && q.day < min"
          @click="pick(q.day)"
        >
          {{ q.label }}
        </button>
      </div>

      <div class="cal">
        <div v-if="view === 'days'" class="cal-head">
          <button class="icon-btn" aria-label="Previous month" @click="shiftMonth(-1)"><ChevronLeft :size="22" /></button>
          <button class="jump" aria-label="Pick a month" @click="view = 'months'">{{ monthLabel }} <ChevronDown :size="16" /></button>
          <button class="icon-btn" aria-label="Next month" @click="shiftMonth(1)"><ChevronRight :size="22" /></button>
        </div>
        <div v-else-if="view === 'months'" class="cal-head">
          <button class="icon-btn" aria-label="Previous year" @click="shiftYear(-1)"><ChevronLeft :size="22" /></button>
          <button class="jump" aria-label="Pick a year" @click="openYears">{{ year }} <ChevronDown :size="16" /></button>
          <button class="icon-btn" aria-label="Next year" @click="shiftYear(1)"><ChevronRight :size="22" /></button>
        </div>
        <div v-else class="cal-head">
          <button class="icon-btn" aria-label="Earlier years" @click="yearStart -= 12"><ChevronLeft :size="22" /></button>
          <span>{{ yearStart }} – {{ yearStart + 11 }}</span>
          <button class="icon-btn" aria-label="Later years" @click="yearStart += 12"><ChevronRight :size="22" /></button>
        </div>

        <div v-if="view === 'days'" class="grid">
          <span v-for="(w, i) in weekdays" :key="'w' + i" class="wd">{{ w }}</span>
          <button
            v-for="c in cells"
            :key="c.d"
            class="cell"
            :class="{ out: c.out, today: c.d === today, ink: c.d === day }"
            :disabled="c.past"
            @click="pick(c.d)"
          >
            {{ c.n }}
          </button>
        </div>
        <div v-else-if="view === 'months'" class="grid wide">
          <button
            v-for="m in months"
            :key="m.key"
            class="cell"
            :class="{ today: m.key === today.slice(0, 7), ink: m.key === day.slice(0, 7) }"
            :disabled="m.past"
            @click="pickMonth(m.key)"
          >
            {{ m.label }}
          </button>
        </div>
        <div v-else class="grid wide">
          <button
            v-for="y in years"
            :key="y.y"
            class="cell"
            :class="{ today: y.y === Number(today.slice(0, 4)), ink: y.y === Number(day.slice(0, 4)) }"
            :disabled="y.past"
            @click="pickYear(y.y)"
          >
            {{ y.y }}
          </button>
        </div>
      </div>

      <footer class="foot">
        <span class="summary" :class="{ error: tooEarly }">{{ tooEarly ? 'Too early' : shortDay(day) }}</span>
        <button v-if="allowClear" class="btn" @click="clear">{{ clearLabel }}</button>
        <button class="btn primary" :disabled="tooEarly" @click="done"><Check :size="18" /> {{ doneLabel }}</button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 60;
  background: oklch(0 0 0 / 22%);
  display: flex;
  align-items: flex-end;
  animation: fade-in 0.15s;
}
.sheet {
  width: calc(100% - 16px);
  max-width: 520px;
  margin: 0 auto 8px;
  border-radius: 22px;
  padding: 8px 14px calc(14px + env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-height: 92vh;
  overflow-y: auto;
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-left: 4px;
}
.head b {
  font-size: 1.05rem;
}
.chips {
  display: flex;
  gap: 8px;
  overflow-x: auto;
  scrollbar-width: none;
  padding: 2px;
}
.chip:disabled {
  opacity: 0.35;
}
.cal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-weight: 500;
}
.jump {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-weight: 500;
  padding: 6px 10px;
  border-radius: 10px;
}
.grid.wide {
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
}
.grid.wide .cell {
  height: 48px;
}
.grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 2px;
  text-align: center;
}
.wd {
  font-size: 0.75rem;
  color: var(--muted);
  padding: 4px 0;
}
.cell {
  height: 38px;
  border-radius: 10px;
  font-variant-numeric: tabular-nums;
  border: 1px solid transparent;
}
.cell.out {
  color: var(--muted);
  opacity: 0.6;
}
.cell.today:not(.ink) {
  background: var(--glass-control);
  box-shadow:
    inset 0 1px 0 var(--glass-control-rim),
    inset 0 0 0 1px var(--glass-edge);
  font-weight: 600;
}
.cell:disabled {
  opacity: 0.25;
}
.cell:active:not(:disabled):not(.ink) {
  background: var(--glass-state);
}
.foot {
  display: flex;
  align-items: center;
  gap: 8px;
}
.summary {
  flex: 1;
  font-weight: 500;
}
</style>
