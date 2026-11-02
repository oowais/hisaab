<script setup lang="ts">
// Snooze a subscription's reminder: the same quick picks as the Android
// overlay (from core), or any day at the reminder time.
import { BellRing, X } from 'lucide-vue-next'
import type { SnoozePick } from '~/composables/api'

const props = defineProps<{ id: string; name: string }>()
const emit = defineEmits<{ close: []; done: [message: string] }>()
useBackClose(() => emit('close'))
const drag = useSheetDrag(() => emit('close'))

const picks = ref<SnoozePick[]>([])
onMounted(async () => (picks.value = await api.snoozePicks().catch(() => [])))
const pickingDay = ref(false)
const error = ref('')

const time = (ms: number) => new Date(ms).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
const label = (ms: number) => {
  const day = isoDay(new Date(ms))
  return day === todayIso() ? time(ms) : `${shortDay(day)} ${time(ms)}`
}

async function snooze(atMs: number) {
  try {
    await api.snooze(props.id, atMs)
    emit('done', `Reminder again ${label(atMs)}`)
  } catch (e) {
    error.value = friendlyError(e, 'snooze')
  }
}
async function snoozeTo(day: string | null) {
  if (day) await snooze(await api.reminderAt(day))
}
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <div
      class="sheet popover"
      :class="{ dragging: drag.dragging.value }"
      :style="drag.style.value"
      v-on="drag.handlers"
      role="dialog"
      aria-label="Snooze reminder"
    >
      <header class="head">
        <b><BellRing :size="18" /> Remind me again · {{ name }}</b>
        <button class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>
      <div class="picks">
        <button v-for="p in picks" :key="p.label" class="pane pick" @click="snooze(p.atMs)">
          <b>{{ p.label }}</b>
          <span class="muted small">{{ label(p.atMs) }}</span>
        </button>
      </div>
      <button class="btn" @click="pickingDay = true">Pick a day</button>
      <p v-if="error" class="error">{{ error }}</p>
    </div>
    <DateSheet
      v-if="pickingDay"
      :model-value="null"
      title="Remind me on"
      :min="addDays(todayIso(), 1)"
      done-label="Snooze"
      @update:model-value="snoozeTo"
      @close="pickingDay = false"
    />
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 50;
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
.head b {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}
.picks {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.pick {
  display: flex;
  flex-direction: column;
  gap: 2px;
  align-items: flex-start;
  padding: 12px 14px;
  text-align: left;
}
.error {
  margin: 0;
}
</style>
