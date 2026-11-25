<script setup lang="ts">
// A reminder's Pay now, from a desktop notification (Linux: the window opens
// here): that one charge paid today. Snoozing or ignoring a reminder never
// moves a charge; only this does.
import { Wallet, X } from 'lucide-vue-next'
import type { ChargeRef, Detail, Paid } from '~/composables/api'

const props = defineProps<{ sub: Detail; charge: ChargeRef }>()
const emit = defineEmits<{ close: []; done: [paid: Paid] }>()
useBackClose(() => emit('close'))
const drag = useSheetDrag(() => emit('close'))

const today = todayIso()
// The charge as it stands (coming, or already due when paid late).
const found = computed(() =>
  [...props.sub.coming, ...props.sub.history].find(
    (c) => c.periodId === props.charge.periodId && c.scheduledDate === props.charge.scheduledDate,
  ),
)
const busy = ref(false)
const error = ref('')

async function pay() {
  if (busy.value) return
  busy.value = true
  try {
    const paid = await api.payNow(props.sub.id, props.charge)
    changed()
    emit('done', paid)
  } catch (e) {
    error.value = friendlyError(e, 'mark it paid')
  } finally {
    busy.value = false
  }
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
      aria-label="Pay now"
    >
      <header class="head">
        <b><Wallet :size="18" /> Pay now · {{ sub.name }}</b>
        <button class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>
      <div v-if="found" class="pane charge">
        <b>{{ formatMoney(found.amountMinor, found.currency) }}</b>
        <span class="muted small">Due {{ shortDay(found.dueDate, today) }} · {{ relativeDay(found.dueDate, today) }}</span>
      </div>
      <p class="muted small note">Marks this charge paid today. The next ones stay on their usual days.</p>
      <button class="btn primary" :disabled="busy" @click="pay">Paid today</button>
      <p v-if="error" class="error">{{ error }}</p>
    </div>
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
.charge {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 12px 14px;
}
.note,
.error {
  margin: 0;
  padding-left: 4px;
}
</style>
