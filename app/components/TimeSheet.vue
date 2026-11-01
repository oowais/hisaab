<script setup lang="ts">
// Time picker: hour and minute wheels from Befikr's DateTimeSheet (mode
// "time"). Emits "HH:MM". Never the native picker.
import { Check, X } from 'lucide-vue-next'

const props = withDefaults(defineProps<{ modelValue: string; title?: string }>(), { title: 'Time' })
const emit = defineEmits<{ 'update:modelValue': [hm: string]; close: [] }>()
useBackClose(() => emit('close'))

const hour = ref(Number(props.modelValue.slice(0, 2)) || 0)
const minute = ref((Math.round(Number(props.modelValue.slice(3, 5)) / 5) * 5) % 60)

// ---- Wheels (scroll-snap columns)
const ITEM = 40
const hours = Array.from({ length: 24 }, (_, i) => i)
const minutes = Array.from({ length: 12 }, (_, i) => i * 5)
const hourWheel = ref<HTMLElement>()
const minuteWheel = ref<HTMLElement>()
const pad = (n: number) => String(n).padStart(2, '0')

let programmatic = false
function scrollWheels(smooth = false) {
  programmatic = true
  const behavior = smooth ? 'smooth' : 'instant'
  hourWheel.value?.scrollTo({ top: hour.value * ITEM, behavior: behavior as ScrollBehavior })
  minuteWheel.value?.scrollTo({ top: (minute.value / 5) * ITEM, behavior: behavior as ScrollBehavior })
  setTimeout(() => (programmatic = false), smooth ? 400 : 50)
}
let settle: ReturnType<typeof setTimeout> | undefined
function onWheelScroll() {
  if (programmatic) return
  clearTimeout(settle)
  settle = setTimeout(() => {
    if (hourWheel.value) hour.value = Math.min(23, Math.max(0, Math.round(hourWheel.value.scrollTop / ITEM)))
    if (minuteWheel.value) minute.value = Math.min(55, Math.max(0, Math.round(minuteWheel.value.scrollTop / ITEM) * 5))
  }, 90)
}
function pickTime(h: number, m: number) {
  hour.value = h
  minute.value = m
  scrollWheels(true)
}
onMounted(() => nextTick(() => scrollWheels()))

function done() {
  emit('update:modelValue', `${pad(hour.value)}:${pad(minute.value)}`)
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
      <div class="wheels">
        <div class="band" />
        <div ref="hourWheel" class="wheel" @scroll.passive="onWheelScroll">
          <div v-for="h in hours" :key="h" class="item" :class="{ on: h === hour }" @click="pickTime(h, minute)">{{ pad(h) }}</div>
        </div>
        <span class="colon">:</span>
        <div ref="minuteWheel" class="wheel" @scroll.passive="onWheelScroll">
          <div v-for="m in minutes" :key="m" class="item" :class="{ on: m === minute }" @click="pickTime(hour, m)">{{ pad(m) }}</div>
        </div>
      </div>
      <footer class="foot">
        <span class="summary">{{ pad(hour) }}:{{ pad(minute) }}</span>
        <button class="btn primary" @click="done"><Check :size="18" /> Done</button>
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
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-left: 4px;
}
.wheels {
  position: relative;
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 4px;
  height: 200px;
}
.band {
  position: absolute;
  left: 0;
  right: 0;
  top: 80px;
  height: 40px;
  border-radius: var(--radius-control);
  background: var(--glass-state);
  box-shadow:
    inset 0 1px 0 var(--glass-state-rim),
    0 0 0 1px var(--glass-state-edge);
  pointer-events: none;
}
.wheel {
  position: relative;
  width: 84px;
  height: 200px;
  overflow-y: auto;
  scroll-snap-type: y mandatory;
  padding: 80px 0;
  scrollbar-width: none;
  overscroll-behavior: contain;
  mask-image: linear-gradient(transparent, #000 30%, #000 70%, transparent);
}
.wheel::-webkit-scrollbar {
  display: none;
}
.item {
  height: 40px;
  line-height: 40px;
  text-align: center;
  scroll-snap-align: center;
  font-size: 1.35rem;
  font-variant-numeric: tabular-nums;
  color: var(--muted);
}
.item.on {
  color: var(--text);
  font-weight: 600;
}
.colon {
  font-size: 1.35rem;
  font-weight: 600;
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
