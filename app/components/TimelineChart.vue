<script setup lang="ts">
// The Timeline chart, hand-built SVG: one lane per subscription, x = time.
// Bars by status: active filled, trial light with a dashed edge, paused a
// dashed outline only; future periods lighter. A diamond at every price or
// plan change, a dashed line for today. The icon column stays put while the
// chart scrolls; names follow the scroll so they stay readable.
// Zoom: the presets from the page, pinch (two fingers) and ctrl+scroll,
// keeping the point under the fingers / pointer in place.
import type { TimelineLane } from '~/composables/api'
import { domain, priceChanges, xOf, type Marker } from '~/utils/timeline'

const props = defineProps<{ lanes: TimelineLane[]; today: string; zoom: '6m' | '1y' | 'all' }>()
const emit = defineEmits<{ open: [subId: string, periodId: string]; marker: [m: Marker] }>()

const LANE = 52
const AXIS = 28
const BAR = 18

const scroller = ref<HTMLElement>()
const width = ref(320)
const scrollLeft = ref(0)
let resize: ResizeObserver | undefined
onMounted(() => {
  resize = new ResizeObserver(() => (width.value = scroller.value?.clientWidth ?? 320))
  if (scroller.value) resize.observe(scroller.value)
})
onUnmounted(() => resize?.disconnect())

const span = computed(() =>
  domain(
    props.lanes.flatMap((l) => l.periods),
    props.today,
  ),
)
const totalDays = computed(() => daysBetween(span.value.start, span.value.end) + 1)
const fitPx = computed(() => width.value / totalDays.value)
const presetPx = (z: typeof props.zoom) => (z === 'all' ? fitPx.value : width.value / (z === '1y' ? 365 : 182))
const pxPerDay = ref(presetPx(props.zoom))
const minPx = computed(() => Math.min(fitPx.value, width.value / 365))
const maxPx = computed(() => width.value / 21)
const chartWidth = computed(() => Math.max(width.value, totalDays.value * pxPerDay.value))
const height = computed(() => AXIS + props.lanes.length * LANE + 8)
const x = (day: string) => xOf(day, span.value.start, pxPerDay.value)

/** Zoom to `px` per day, keeping the chart point at `anchor` (px from the
 *  scroller's left edge) where it is. */
function zoomTo(px: number, anchor = width.value * 0.8) {
  const el = scroller.value
  const clamped = Math.min(maxPx.value, Math.max(minPx.value, px))
  if (!el) return void (pxPerDay.value = clamped)
  const day = (el.scrollLeft + anchor) / pxPerDay.value
  pxPerDay.value = clamped
  nextTick(() => (el.scrollLeft = day * clamped - anchor))
}
/** Presets: today ~80% across, so recent history and what's next both show. */
function applyPreset() {
  pxPerDay.value = Math.max(minPx.value, presetPx(props.zoom))
  nextTick(() => {
    if (scroller.value) scroller.value.scrollLeft = x(props.today) - width.value * 0.8
  })
}
watch(() => props.zoom, applyPreset)
watch(width, applyPreset, { once: true })
onMounted(() => nextTick(applyPreset))

// ---- Ctrl+scroll (desktop) and pinch (touch)
function wheel(e: WheelEvent) {
  if (!e.ctrlKey) return
  e.preventDefault()
  const left = scroller.value!.getBoundingClientRect().left
  zoomTo(pxPerDay.value * Math.exp(-e.deltaY * 0.002), e.clientX - left)
}
const touches = new Map<number, number>()
let pinch: { dist: number; px: number } | null = null
function down(e: PointerEvent) {
  if (e.pointerType !== 'touch') return
  touches.set(e.pointerId, e.clientX)
  if (touches.size === 2) {
    const [a, b] = [...touches.values()]
    pinch = { dist: Math.max(20, Math.abs(a! - b!)), px: pxPerDay.value }
  }
}
function move(e: PointerEvent) {
  if (!touches.has(e.pointerId)) return
  touches.set(e.pointerId, e.clientX)
  if (!pinch || touches.size !== 2) return
  const [a, b] = [...touches.values()]
  const left = scroller.value!.getBoundingClientRect().left
  zoomTo(pinch.px * (Math.max(20, Math.abs(a! - b!)) / pinch.dist), (a! + b!) / 2 - left)
}
function up(e: PointerEvent) {
  touches.delete(e.pointerId)
  if (touches.size < 2) pinch = null
}

// ---- Axis: months, or quarters / years when zoomed out
const ticks = computed(() => {
  const perMonth = pxPerDay.value * 30.4
  const step = perMonth >= 44 ? 1 : perMonth >= 16 ? 3 : 12
  const out: { x: number; label: string; year: boolean }[] = []
  const d = parseDay(span.value.start)
  d.setDate(1)
  d.setMonth(d.getMonth() + 1)
  while (isoDay(d) <= span.value.end) {
    const m = d.getMonth()
    if (m % step === 0) {
      out.push({
        x: x(isoDay(d)),
        label: m === 0 ? String(d.getFullYear()) : d.toLocaleDateString(undefined, { month: 'short' }),
        year: m === 0,
      })
    }
    d.setMonth(d.getMonth() + 1)
  }
  return out
})

// ---- Lanes
const rows = computed(() =>
  props.lanes.map((l, i) => {
    const y = AXIS + i * LANE
    const color = l.color || fallbackColor(l.name)
    const bars = l.periods.map((p) => {
      const x0 = x(p.startDate)
      const x1 = p.endDate ? x(addDays(p.endDate, 1)) : chartWidth.value
      return { p, x: x0, w: Math.max(2, x1 - x0), future: p.startDate > props.today }
    })
    const first = bars[0]?.x ?? 0
    return { l, y, color, bars, first, markers: priceChanges(l.periods).map((m) => ({ ...m, x: x(m.date) })) }
  }),
)
// Names stick to the left edge while their lane scrolls under it.
const nameX = (first: number) => Math.max(first, scrollLeft.value) + 4
</script>

<template>
  <div class="chart">
    <div class="icons" :style="{ paddingTop: `${AXIS}px` }">
      <button
        v-for="r in rows"
        :key="r.l.id"
        class="icon"
        :style="{ height: `${LANE}px` }"
        :aria-label="r.l.name"
        @click="emit('open', r.l.id, r.l.current?.id ?? '')"
      >
        <SubIcon :icon="r.l.icon" :color="r.l.color" :name="r.l.name" :size="32" />
      </button>
    </div>
    <div
      ref="scroller"
      class="scroller"
      @scroll.passive="scrollLeft = ($event.target as HTMLElement).scrollLeft"
      @wheel="wheel"
      @pointerdown="down"
      @pointermove="move"
      @pointerup="up"
      @pointercancel="up"
    >
      <svg :width="chartWidth" :height="height" role="img" aria-label="Timeline of your subscriptions">
        <g class="axis">
          <g v-for="t in ticks" :key="t.x">
            <line :x1="t.x" :x2="t.x" :y1="AXIS - 6" :y2="height" class="grid" :class="{ year: t.year }" />
            <text :x="t.x + 4" :y="AXIS - 10" :class="{ year: t.year }">{{ t.label }}</text>
          </g>
        </g>

        <g v-for="r in rows" :key="r.l.id" :style="{ '--c': r.color }">
          <text :x="nameX(r.first)" :y="r.y + 15" class="name">{{ r.l.name }}</text>
          <g
            v-for="b in r.bars"
            :key="b.p.id"
            class="bar"
            :class="[b.p.status, { future: b.future }]"
            @click="emit('open', r.l.id, b.p.id)"
          >
            <title>{{ b.p.planLabel ?? '' }} {{ shortDay(b.p.startDate) }} – {{ b.p.endDate ? shortDay(b.p.endDate) : 'now' }}</title>
            <rect :x="b.x" :y="r.y + 24" :width="b.w" :height="BAR" rx="6" />
          </g>
          <g v-for="m in r.markers" :key="m.date" class="marker" @click.stop="emit('marker', m)">
            <title>{{ m.text }}</title>
            <!-- A bigger invisible target than the diamond itself. -->
            <rect :x="m.x - 14" :y="r.y + 19" width="28" height="28" fill="transparent" />
            <rect :x="m.x - 6" :y="r.y + 27" width="12" height="12" :transform="`rotate(45 ${m.x} ${r.y + 33})`" />
          </g>
        </g>

        <line :x1="x(today)" :x2="x(today)" :y1="AXIS - 6" :y2="height" class="today" />
      </svg>
    </div>
  </div>
</template>

<style scoped>
.chart {
  display: flex;
  margin: 0 -16px;
}
.icons {
  flex: none;
  width: 52px;
  display: flex;
  flex-direction: column;
  align-items: center;
  position: relative;
  z-index: 1;
  background: var(--bg);
  box-shadow: 1px 0 0 var(--line);
}
.icon {
  display: grid;
  place-items: center;
  padding: 0;
  width: 100%;
}
.scroller {
  flex: 1;
  min-width: 0;
  overflow-x: auto;
  overflow-y: hidden;
  touch-action: pan-x pan-y;
  overscroll-behavior-x: contain;
}
svg {
  display: block;
  font-family: inherit;
}
.axis text {
  font-size: 11px;
  fill: var(--muted);
}
.axis text.year {
  font-weight: 600;
  fill: var(--text);
}
.grid {
  stroke: var(--line);
  stroke-width: 1;
}
.grid.year {
  stroke: color-mix(in oklch, var(--muted) 45%, transparent);
}
.name {
  font-size: 12px;
  fill: var(--text);
  pointer-events: none;
}
.bar {
  cursor: pointer;
}
.bar rect {
  fill: var(--c);
  /* A faint edge, so near-black brands still show on the dark theme. */
  stroke: color-mix(in oklch, var(--text) 22%, transparent);
  stroke-width: 1;
}
.bar.trial rect {
  fill: color-mix(in oklch, var(--c) 30%, transparent);
  stroke: var(--c);
  stroke-width: 1.5;
  stroke-dasharray: 4 3;
}
.bar.paused rect {
  fill: transparent;
  stroke: var(--c);
  stroke-width: 1.5;
  stroke-dasharray: 4 3;
}
.bar.future {
  opacity: 0.4;
}
.marker {
  cursor: pointer;
}
.marker rect:last-child {
  fill: var(--bg);
  stroke: var(--text);
  stroke-width: 2;
}
.today {
  stroke: var(--red);
  stroke-width: 1.5;
  stroke-dasharray: 4 4;
}
</style>
