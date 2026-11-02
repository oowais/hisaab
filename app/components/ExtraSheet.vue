<script setup lang="ts">
// A one-time charge on a payment, besides its schedule (a setup fee, an
// extra member, a repair): what for, how much, which day. Add as many as
// needed; edit or delete one (with undo). Counts as spend; no reminder.
import { CalendarClock, Trash2, X } from 'lucide-vue-next'
import type { ExtraCharge } from '~/composables/api'

const props = defineProps<{
  subscriptionId: string
  name: string
  /** The payment's currency, for a new one. */
  currency: string
  /** Editing this one; none = add a new one. */
  extra?: ExtraCharge | null
  /** A new one starting from these (duplicating a charge). */
  prefill?: { dueDate: string; amountMinor: number; currency: string; label: string } | null
}>()
const emit = defineEmits<{ close: [] }>()
const toast = useToast()
useBackClose(() => emit('close'))
const drag = useSheetDrag(() => emit('close'))

const today = todayIso()
const e = props.extra
const from = e ?? props.prefill ?? null
const label = ref(from?.label ?? '')
const currency = ref(from?.currency ?? props.currency)
const amountText = ref(from ? amountTextOf(from.amountMinor, from.currency) : '')
const day = ref(from?.dueDate ?? today)
const pickingCurrency = ref(false)
const pickingDay = ref(false)
const error = ref('')
const busy = ref(false)
const amountInput = ref<HTMLInputElement>()
onMounted(() => {
  if (!e) amountInput.value?.focus()
})

function amountTextOf(minor: number, cur: string) {
  return (minor / 10 ** minorDigits(cur)).toFixed(minorDigits(cur))
}

async function save() {
  const amount = parseAmount(amountText.value, currency.value)
  if (!amount) {
    error.value = 'Enter the amount.'
    return amountInput.value?.focus()
  }
  busy.value = true
  error.value = ''
  const fields = { dueDate: day.value, amountMinor: amount, currency: currency.value, label: label.value.trim() }
  try {
    if (e) {
      await api.updateExtra({ ...e, ...fields })
      const before = e
      changed()
      toast.show({ message: 'Saved', action: { label: 'Undo', run: () => void api.updateExtra(before).then(changed) } })
    } else {
      const made = await api.addExtra({ subscriptionId: props.subscriptionId, ...fields })
      changed()
      toast.show({
        message: `Added ${formatMoney(amount, currency.value)} ${relativeDay(day.value, today)}`,
        action: { label: 'Undo', run: () => void api.deleteExtra(made.id).then(changed) },
      })
    }
    emit('close')
  } catch (err) {
    error.value = friendlyError(err, 'save the charge')
  } finally {
    busy.value = false
  }
}

async function remove() {
  if (!e) return
  busy.value = true
  try {
    await api.deleteExtra(e.id)
    changed()
    emit('close')
    toast.show({ message: 'One-time charge deleted', action: { label: 'Undo', run: () => void api.restoreExtra(e.id).then(changed) } })
  } catch (err) {
    error.value = friendlyError(err, 'delete it')
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <form
      class="sheet popover"
      :class="{ dragging: drag.dragging.value }"
      :style="drag.style.value"
      v-on="drag.handlers"
      role="dialog"
      :aria-label="`One-time charge · ${name}`"
      @submit.prevent="save"
    >
      <header class="head">
        <b>{{ name }} · {{ e ? 'one-time charge' : 'add a one-time charge' }}</b>
        <button type="button" class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>

      <div class="amount-row">
        <input ref="amountInput" v-model="amountText" class="field amount" inputmode="decimal" placeholder="0.00" aria-label="Amount" />
        <button type="button" class="field currency" aria-label="Currency" @click="pickingCurrency = true">{{ currency }}</button>
      </div>
      <input v-model="label" class="field" placeholder="What for (setup fee, extra member…)" aria-label="What for" maxlength="80" />

      <button type="button" class="pick" @click="pickingDay = true">
        <span class="muted small">Charged on</span>
        <span>
          <CalendarClock :size="15" /> {{ shortDay(day, today) }} <span class="muted">· {{ relativeDay(day, today) }}</span>
        </span>
      </button>

      <p v-if="error" class="error">{{ error }}</p>
      <button type="submit" class="btn primary" :disabled="busy">{{ e ? 'Save' : 'Add' }}</button>
      <button v-if="e" type="button" class="btn danger" :disabled="busy" @click="remove"><Trash2 :size="16" /> Delete</button>
    </form>
    <CurrencySheet v-if="pickingCurrency" v-model="currency" @close="pickingCurrency = false" />
    <DateSheet v-if="pickingDay" v-model="day" title="Charged on" @close="pickingDay = false" />
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
.amount-row {
  display: flex;
  gap: 8px;
}
.amount {
  flex: 1;
  font-size: 1.3rem;
  font-variant-numeric: tabular-nums;
}
.currency {
  width: auto;
  flex: none;
  padding: 0 16px;
  font-weight: 600;
}
.pick {
  display: flex;
  flex-direction: column;
  gap: 2px;
  align-items: flex-start;
  padding: 8px 4px;
  text-align: left;
}
.pick span:last-child {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.error {
  margin: 0;
}
</style>
