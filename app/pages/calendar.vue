<script setup lang="ts">
// A month at a glance: each day shows what's charged on it, the month's
// total on top. Tap a day for its charges (they open, move and change like
// on Upcoming); arrows or a swipe for other months.
import { ChevronLeft, ChevronRight } from 'lucide-vue-next'
import type { MonthCharges, SubscriptionView, UpcomingCharge } from '~/composables/api'

const router = useRouter()
const toast = useToast()
const today = todayIso()
const month = ref(today.slice(0, 7)) // "YYYY-MM"
const data = ref<MonthCharges | null>(null)
const subs = ref<Map<string, SubscriptionView>>(new Map())
const main = ref('EUR')
const picked = ref<string | null>(today)
const error = ref('')

async function load() {
  try {
    const [m, o] = await Promise.all([api.monthCharges(`${month.value}-01`, today), api.overview(today)])
    data.value = m
    subs.value = new Map(o.subscriptions.map((s) => [s.id, s]))
    main.value = o.mainCurrency
    error.value = ''
  } catch (e) {
    error.value = friendlyError(e, 'load the month')
  }
}
onMounted(load)
watch(dataVersion, load)
watch(month, load)

function shift(n: number) {
  const d = parseDay(`${month.value}-01`)
  d.setMonth(d.getMonth() + n)
  month.value = isoDay(d).slice(0, 7)
  picked.value = month.value === today.slice(0, 7) ? today : null
}
const title = computed(() => parseDay(`${month.value}-01`).toLocaleDateString(undefined, { month: 'long', year: 'numeric' }))

// Weeks from Monday; days of other months are blank.
const weekdays = computed(() => {
  const monday = parseDay('2024-01-01') // a Monday
  return Array.from({ length: 7 }, (_, i) =>
    new Date(monday.getTime() + i * 86_400_000).toLocaleDateString(undefined, { weekday: 'narrow' }),
  )
})
const byDay = computed(() => {
  const m = new Map<string, UpcomingCharge[]>()
  for (const c of data.value?.charges ?? []) m.set(c.dueDate, [...(m.get(c.dueDate) ?? []), c])
  return m
})
const cells = computed(() => {
  const first = parseDay(`${month.value}-01`)
  const lead = (first.getDay() + 6) % 7
  const days = new Date(first.getFullYear(), first.getMonth() + 1, 0).getDate()
  const out: ({ day: string; n: number; charges: UpcomingCharge[] } | null)[] = Array(lead).fill(null)
  for (let n = 1; n <= days; n++) {
    const day = `${month.value}-${String(n).padStart(2, '0')}`
    out.push({ day, n, charges: byDay.value.get(day) ?? [] })
  }
  return out
})
const dayCharges = computed(() => (picked.value ? (byDay.value.get(picked.value) ?? []) : []))

// Swipe left / right for the next / previous month.
let startX: number | null = null
const down = (e: PointerEvent) => (startX = e.clientX)
function up(e: PointerEvent) {
  if (startX == null) return
  const dx = e.clientX - startX
  startX = null
  if (Math.abs(dx) > 60) shift(dx < 0 ? 1 : -1)
}

// ---- Row actions, as on Upcoming
const moving = ref<UpcomingCharge | null>(null)
const editing = ref<UpcomingCharge | null>(null)
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
const open = (id: string) => router.push(`/sub/${id}`)
</script>

<template>
  <Screen :title="title">
    <template #actions>
      <button class="icon-btn" aria-label="Previous month" @click="shift(-1)"><ChevronLeft :size="22" /></button>
      <button class="icon-btn" aria-label="Next month" @click="shift(1)"><ChevronRight :size="22" /></button>
    </template>
    <p v-if="error" class="error">{{ error }}</p>

    <template v-if="data">
      <p class="total">
        <b>{{ formatMoney(data.total, main) }}</b>
        <span class="muted small"> this month</span>
        <span v-if="data.excludedCurrencies.length" class="muted small"> · without {{ data.excludedCurrencies.join(', ') }}</span>
      </p>

      <div class="grid" role="grid" :aria-label="title" @pointerdown="down" @pointerup="up">
        <span v-for="(w, i) in weekdays" :key="`w${i}`" class="wd muted">{{ w }}</span>
        <template v-for="(c, i) in cells" :key="c?.day ?? `b${i}`">
          <span v-if="!c" class="cell blank" />
          <button
            v-else
            type="button"
            class="cell"
            :class="{ today: c.day === today, picked: c.day === picked, past: c.day < today }"
            :aria-label="`${shortDay(c.day)}, ${c.charges.length} ${c.charges.length === 1 ? 'charge' : 'charges'}`"
            @click="picked = c.day"
          >
            <span class="n">{{ c.n }}</span>
            <span class="icons">
              <SubIcon
                v-for="ch in c.charges.slice(0, 3)"
                :key="`${ch.periodId}-${ch.scheduledDate}`"
                :icon="subs.get(ch.subscriptionId)?.icon ?? null"
                :color="subs.get(ch.subscriptionId)?.color ?? null"
                :name="subs.get(ch.subscriptionId)?.name ?? '?'"
                :size="16"
              />
              <span v-if="c.charges.length > 3" class="more muted">+{{ c.charges.length - 3 }}</span>
            </span>
          </button>
        </template>
      </div>

      <template v-if="picked">
        <h2 class="section-title">
          {{
            relativeDay(picked, today) === shortDay(picked, today)
              ? shortDay(picked, today)
              : `${shortDay(picked, today)} · ${relativeDay(picked, today)}`
          }}
        </h2>
        <ul v-if="dayCharges.length" class="pane rows list">
          <template v-for="c in dayCharges" :key="`${c.periodId}-${c.scheduledDate}`">
            <ChargeRow
              v-if="subs.get(c.subscriptionId)"
              :charge="c"
              :sub="subs.get(c.subscriptionId)!"
              :main="main"
              :today="today"
              @open="open(c.subscriptionId)"
              @move="moving = c"
              @menu="editing = c"
            />
          </template>
        </ul>
        <p v-else class="muted small">Nothing charged.</p>
      </template>
    </template>

    <DateSheet
      v-if="moving"
      :model-value="moving.dueDate"
      title="Move this charge"
      done-label="Move"
      @update:model-value="moveTo"
      @close="moving = null"
    />
    <ChargeSheet v-if="editing" :charge="editing" :name="subs.get(editing.subscriptionId)?.name ?? ''" @close="editing = null" />
  </Screen>
</template>

<style scoped>
.total {
  margin: 0 0 12px;
}
.total b {
  font-size: 1.3rem;
  font-variant-numeric: tabular-nums;
}
.grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 4px;
  touch-action: pan-y;
}
.wd {
  text-align: center;
  font-size: 0.75rem;
  padding-bottom: 2px;
}
.cell {
  aspect-ratio: 1 / 1.1;
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-start;
  gap: 3px;
  padding-top: 5px;
  min-width: 0;
  color: inherit;
}
.cell.blank {
  pointer-events: none;
}
.cell.past .n {
  color: var(--muted);
}
.cell.today .n {
  background: var(--text);
  color: var(--bg);
  border-radius: 999px;
  padding: 0 6px;
}
.cell.picked {
  box-shadow: inset 0 0 0 1.5px var(--text);
}
.n {
  font-size: 0.85rem;
  font-variant-numeric: tabular-nums;
  line-height: 1.4;
}
.icons {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 2px;
}
.more {
  font-size: 0.65rem;
}
.list {
  margin: 0;
}
</style>
