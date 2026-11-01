<script setup lang="ts">
// One upcoming charge. Tap opens the subscription; swipe left moves this
// charge to another day; long-press / right-click opens the menu.
import { CalendarArrowUp } from 'lucide-vue-next'
import type { SubscriptionView, UpcomingCharge } from '~/composables/api'

const props = defineProps<{ charge: UpcomingCharge; sub: SubscriptionView; main: string; today: string }>()
const emit = defineEmits<{ open: []; move: []; menu: [e: MouseEvent] }>()

const when = computed(() => relativeDay(props.charge.dueDate, props.today))
const original = computed(() => formatMoney(props.charge.amountMinor, props.charge.currency))
// Shared: your part of it (what the totals count).
const mine = computed(() =>
  props.sub.myShare == null ? '' : formatMoney(Math.round((props.charge.amountMinor * props.sub.myShare) / 100), props.charge.currency),
)
const converted = computed(() =>
  props.charge.currency !== props.main && props.charge.inMain != null ? formatMoney(props.charge.inMain, props.main) : null,
)

// ---- Swipe left (Befikr's TaskRow, one direction)
const AXIS_LOCK = 10
const el = ref<HTMLElement>()
const dx = ref(0)
const dragging = ref(false)
let startX = 0
let startY = 0
let axis: 'x' | 'y' | null = null
let armed = false
let swiped = false
const threshold = () => Math.min(110, (el.value?.offsetWidth ?? 360) * 0.3)
const past = computed(() => -dx.value >= threshold())

function down(e: PointerEvent) {
  if (e.pointerType === 'mouse' && e.button !== 0) return
  startX = e.clientX
  startY = e.clientY
  axis = null
  armed = false
  swiped = false
  dragging.value = true
}
function move(e: PointerEvent) {
  if (!dragging.value) return
  const x = e.clientX - startX
  const y = e.clientY - startY
  if (!axis) {
    if (Math.abs(x) < AXIS_LOCK && Math.abs(y) < AXIS_LOCK) return
    axis = Math.abs(x) > Math.abs(y) ? 'x' : 'y'
    if (axis === 'y') return (dragging.value = false)
    swiped = true
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }
  let v = x > 0 ? x / 6 : x
  const t = threshold()
  if (-v > t) v = -(t + (-v - t) * 0.4)
  dx.value = v
  const isPast = -v >= t
  if (isPast !== armed) {
    armed = isPast
    if (armed) haptic()
  }
}
function up() {
  if (!dragging.value) return
  dragging.value = false
  dx.value = 0
  if (armed) emit('move')
  armed = false
}
function menu(e: MouseEvent) {
  dragging.value = false
  armed = false
  dx.value = 0
  emit('menu', e)
}
function open() {
  if (swiped) return (swiped = false)
  emit('open')
}
</script>

<template>
  <li ref="el" class="row" @contextmenu="menu">
    <div v-if="dx < 0" class="reveal" :class="{ past }"><CalendarArrowUp :size="22" /></div>
    <div
      class="swipe"
      :class="{ dragging, moved: dx !== 0 }"
      :style="{ transform: dx ? `translateX(${dx}px)` : undefined }"
      @pointerdown="down"
      @pointermove="move"
      @pointerup="up"
      @pointercancel="up"
    >
      <button class="body" @click="open">
        <SubIcon :icon="sub.icon" :color="sub.color" :name="sub.name" :size="38" />
        <span class="main">
          <span class="name">{{ sub.name }}</span>
          <span class="when muted small">
            {{ when }}<template v-if="charge.extraId"> · {{ charge.label ?? 'one-time' }}</template
            ><template v-if="charge.isOverride"> · moved</template><template v-if="charge.estimated"> · about</template>
          </span>
        </span>
        <span class="amount">
          <span>{{ charge.estimated ? '~' : '' }}{{ original }}</span>
          <span v-if="mine" class="muted small">yours {{ mine }}</span>
          <span v-else-if="converted" class="muted small">{{ converted }}</span>
        </span>
      </button>
    </div>
  </li>
</template>

<style scoped>
.row {
  position: relative;
  list-style: none;
  margin: 0 -14px;
  overflow: hidden;
}
.swipe {
  position: relative;
  padding: 8px 14px;
  touch-action: pan-y;
  user-select: none;
  -webkit-user-select: none;
  -webkit-touch-callout: none;
  transition: transform 0.32s cubic-bezier(0.22, 1, 0.36, 1);
}
.swipe.dragging {
  transition: none;
}
.swipe.moved {
  background: var(--popover);
}
.reveal {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 0 22px;
  color: var(--muted);
  background: var(--glass-state);
  transition:
    background-color 0.15s,
    color 0.15s;
}
.reveal.past {
  color: oklch(0.2 0 0);
  background: var(--yellow);
}
.body {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  text-align: left;
}
.main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.amount {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  font-variant-numeric: tabular-nums;
}
</style>
