<script setup lang="ts">
// Fix when a stretch of a payment started or ended: the real start, the
// real cancellation. A side that joins the period before or after (a price
// change, a pause) is fixed by that period and shown as such. Restarting
// after a break stays its own action.
import { CalendarClock, X } from 'lucide-vue-next'
import type { Detail, Period } from '~/composables/api'

const props = defineProps<{ sub: Detail; period: Period }>()
const emit = defineEmits<{ close: []; done: [message: string] }>()
useBackClose(() => emit('close'))

const today = todayIso()
const ps = props.sub.periods
const i = ps.findIndex((p) => p.id === props.period.id)
const prev = ps[i - 1]
const next = ps[i + 1]
const follows = (a: Period | undefined, b: Period | undefined) => !!a?.endDate && !!b && addDays(a.endDate, 1) === b.startDate
const startFixed = follows(prev, props.period)
const endFixed = follows(props.period, next)
const last = !next

const start = ref(props.period.startDate)
const end = ref<string | null>(props.period.endDate)
const picking = ref<'start' | 'end' | null>(null)
const error = ref('')
const busy = ref(false)

async function save() {
  busy.value = true
  error.value = ''
  try {
    await api.setPeriodDates(props.sub.id, props.period.id, start.value, end.value)
    changed()
    emit(
      'done',
      end.value ? `Runs ${shortDay(start.value, today)} – ${shortDay(end.value, today)}` : `Runs from ${shortDay(start.value, today)}`,
    )
  } catch (e) {
    error.value = friendlyError(e, 'change the dates')
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <form class="sheet popover" role="dialog" :aria-label="`Dates · ${sub.name}`" @submit.prevent="save">
      <header class="head">
        <b>Dates · {{ sub.name }}</b>
        <button type="button" class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>

      <button type="button" class="pick" :disabled="startFixed" @click="picking = 'start'">
        <span class="muted small">Started on</span>
        <span><CalendarClock :size="15" /> {{ shortDay(start, today) }}</span>
        <span v-if="startFixed" class="muted small">Follows on from the period before</span>
      </button>
      <button type="button" class="pick" :disabled="endFixed" @click="picking = 'end'">
        <span class="muted small">Ended on</span>
        <span><CalendarClock :size="15" /> {{ end ? shortDay(end, today) : 'Still running' }}</span>
        <span v-if="endFixed" class="muted small">The next period starts right after</span>
      </button>
      <p class="muted small note">Charges follow these dates. Restarting after a break is its own step.</p>

      <p v-if="error" class="error">{{ error }}</p>
      <button type="submit" class="btn primary" :disabled="busy">Save</button>
    </form>
    <DateSheet v-if="picking === 'start'" v-model="start" title="Started on" @close="picking = null" />
    <DateSheet
      v-if="picking === 'end'"
      v-model="end"
      title="Ended on"
      :min="start"
      :allow-clear="last"
      clear-label="Still running"
      @close="picking = null"
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
  gap: 10px;
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-left: 4px;
}
.pick {
  display: flex;
  flex-direction: column;
  gap: 2px;
  align-items: flex-start;
  padding: 8px 4px;
  text-align: left;
}
.pick span:nth-child(2) {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.pick:disabled {
  opacity: 0.6;
}
.note,
.error {
  margin: 0;
}
</style>
