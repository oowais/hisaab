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

// Swipe left / right for the next / previous month: the month follows the
// finger, slides out and the next one slides in; short swipes snap back.
// Vertical moves stay page scrolling (touch-action: pan-y).
const SLOP = 8
const offset = ref(0)
const animate = ref(false)
let start: { x: number; y: number } | null = null
let swiping = false
let sliding = false // a month is sliding out or in
let swallowClick = false
const monthStyle = computed(() => ({
  transform: offset.value ? `translateX(${offset.value}px)` : undefined,
  transition: animate.value ? 'transform 0.18s cubic-bezier(0.22, 1, 0.36, 1)' : 'none',
}))
const settle = () => new Promise((r) => setTimeout(r, 180))

function down(e: PointerEvent) {
  if (sliding || (e.pointerType === 'mouse' && e.button !== 0)) return
  start = { x: e.clientX, y: e.clientY }
  swiping = false
}
function move(e: PointerEvent) {
  if (!start) return
  const dx = e.clientX - start.x
  const dy = e.clientY - start.y
  if (!swiping) {
    if (Math.abs(dx) < SLOP && Math.abs(dy) < SLOP) return
    if (Math.abs(dy) >= Math.abs(dx)) return void (start = null)
    swiping = true
    animate.value = false
    ;(e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId)
  }
  offset.value = dx
}
async function up(e: PointerEvent) {
  start = null
  if (!swiping) return
  swiping = false
  // The release isn't a tap on the day under it.
  swallowClick = true
  setTimeout(() => (swallowClick = false))
  const grid = e.currentTarget as HTMLElement
  const width = grid.offsetWidth
  animate.value = true
  if (Math.abs(offset.value) < Math.min(60, width / 4)) return void (offset.value = 0)
  const dir = offset.value < 0 ? 1 : -1
  sliding = true
  offset.value = -dir * width
  await settle()
  shift(dir)
  animate.value = false
  offset.value = dir * width
  // Lay it out off to the side before it slides in.
  await nextTick()
  void grid.offsetWidth
  animate.value = true
  offset.value = 0
  await settle()
  sliding = false
}
function clickCapture(e: MouseEvent) {
  if (!swallowClick) return
  e.stopPropagation()
  e.preventDefault()
}

// ---- Row actions, as on Upcoming
const moving = ref<UpcomingCharge | null>(null)
const editing = ref<UpcomingCharge | null>(null)
async function moveTo(day: string | null) {
  const c = moving.value
  moving.value = null
  if (c && day && day !== c.dueDate) await moveCharge(c, day)
}
const open = (id: string) => router.push(`/sub/${id}`)
</script>

<template>
  <Screen :title="title" tab>
    <template #actions>
      <button class="icon-btn" aria-label="Previous month" @click="shift(-1)"><ChevronLeft :size="22" /></button>
      <button class="icon-btn" aria-label="Next month" @click="shift(1)"><ChevronRight :size="22" /></button>
      <SettingsLink />
    </template>
    <p v-if="error" class="error">{{ error }}</p>

    <template v-if="data">
      <p class="total">
        <b>{{ formatMoney(data.total, main) }}</b>
        <span class="muted small"> this month</span>
        <span v-if="data.excludedCurrencies.length" class="muted small"> · without {{ data.excludedCurrencies.join(', ') }}</span>
      </p>

      <div class="month">
        <div
          class="grid"
          role="grid"
          :aria-label="title"
          :style="monthStyle"
          @pointerdown="down"
          @pointermove="move"
          @pointerup="up"
          @pointercancel="up"
          @click.capture="clickCapture"
        >
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
          <template v-for="c in dayCharges" :key="chargeKey(c)">
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
    <ExtraSheet
      v-if="editing?.extraId"
      :subscription-id="editing.subscriptionId"
      :name="subs.get(editing.subscriptionId)?.name ?? ''"
      :currency="editing.currency"
      :extra="extraOf(editing)"
      @close="editing = null"
    />
    <ChargeSheet v-else-if="editing" :charge="editing" :name="subs.get(editing.subscriptionId)?.name ?? ''" @close="editing = null" />
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
/* The month slides out of view, not into a sideways scroll. */
.month {
  overflow: clip;
  margin: 0 -16px;
  padding: 0 16px;
}
.grid {
  display: grid;
  user-select: none;
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
