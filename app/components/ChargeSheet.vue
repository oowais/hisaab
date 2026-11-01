<script setup lang="ts">
// One charge, when reality differed from the rule. A past charge: what was
// actually taken (same currency = the charge's amount; another currency =
// what the bank took, e.g. €9.21 for $9.99), or that it wasn't charged. A
// coming one: its amount (a variable bill's real figure), its date, or skip
// it. "Back to the usual" drops the change.
import { CalendarClock, Undo2, X } from 'lucide-vue-next'
import type { Charge, NewOverride } from '~/composables/api'

const props = defineProps<{ charge: Charge; name: string; overrideId?: string | null }>()
const emit = defineEmits<{ close: [] }>()
const toast = useToast()
useBackClose(() => emit('close'))

const today = todayIso()
const past = computed(() => props.charge.status === 'paid')
const c = props.charge
const shownMinor = c.settledAmountMinor ?? c.amountMinor
const currency = ref(c.settledCurrency ?? c.currency)
const amountText = ref(
  c.estimated && !c.isOverride ? '' : (shownMinor / 10 ** minorDigits(currency.value)).toFixed(minorDigits(currency.value)),
)
// A variable bill's estimate, as a hint while its real figure is empty.
const placeholder = (c.amountMinor / 10 ** minorDigits(c.currency)).toFixed(minorDigits(c.currency))
const day = ref(c.dueDate)
const pickingCurrency = ref(false)
const pickingDay = ref(false)
const error = ref('')
const busy = ref(false)

async function save(o: Omit<NewOverride, 'periodId' | 'dueDate'>, message: string) {
  busy.value = true
  error.value = ''
  try {
    const made = await api.setOverride({ periodId: c.periodId, dueDate: c.scheduledDate, ...o })
    changed()
    emit('close')
    // Undo = back to the usual charge (when it was the usual one before).
    const undo = props.overrideId || c.isOverride ? undefined : { label: 'Undo', run: () => void api.deleteOverride(made.id).then(changed) }
    toast.show({ message, action: undo })
  } catch (e) {
    error.value = friendlyError(e, 'save the charge')
  } finally {
    busy.value = false
  }
}

function saveAmount() {
  const amount = parseAmount(amountText.value, currency.value)
  if (amount == null) return void (error.value = 'Enter the amount.')
  const moved = day.value !== c.scheduledDate ? day.value : null
  const label = formatMoney(amount, currency.value)
  // Another currency than the charge's: what the bank took for it.
  if (currency.value !== c.currency) {
    return save({ kind: 'settled', newDueDate: moved, settledAmountMinor: amount, settledCurrency: currency.value }, `Saved ${label}`)
  }
  if (moved && amount === c.amountMinor && !c.estimated) return save({ kind: 'moved', newDueDate: moved }, `Moved to ${relativeDay(moved)}`)
  save({ kind: 'amount_changed', newDueDate: moved, actualAmountMinor: amount, actualCurrency: currency.value }, `Saved ${label}`)
}

const skip = () => save({ kind: 'skipped' }, past.value ? 'Marked as not charged' : 'Skipped this charge')

async function backToUsual() {
  if (!props.overrideId) return
  busy.value = true
  try {
    await api.deleteOverride(props.overrideId)
    changed()
    emit('close')
    toast.show({ message: 'Back to the usual charge' })
  } catch (e) {
    error.value = friendlyError(e, 'undo it')
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <form class="sheet popover" role="dialog" :aria-label="`Charge · ${name}`" @submit.prevent="saveAmount">
      <header class="head">
        <b>{{ name }} · {{ shortDay(c.scheduledDate, today) }}</b>
        <button type="button" class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>

      <span class="muted small">{{ past ? 'Amount taken' : c.estimated ? 'Amount (the bill’s real figure)' : 'Amount' }}</span>
      <div class="amount-row">
        <input v-model="amountText" class="field amount" inputmode="decimal" :placeholder="placeholder" aria-label="Amount" />
        <button type="button" class="field currency" aria-label="Currency" @click="pickingCurrency = true">{{ currency }}</button>
      </div>
      <p v-if="past && currency !== c.currency" class="muted small hint">
        What your bank took for {{ formatMoney(c.amountMinor, c.currency) }}; totals use it.
      </p>

      <button v-if="!past" type="button" class="pick" @click="pickingDay = true">
        <span class="muted small">Charged on</span>
        <span>
          <CalendarClock :size="15" /> {{ shortDay(day) }} <span class="muted">· {{ relativeDay(day, today) }}</span>
        </span>
      </button>

      <p v-if="error" class="error">{{ error }}</p>
      <button type="submit" class="btn primary" :disabled="busy">Save</button>
      <div class="row">
        <button type="button" class="btn" :disabled="busy" @click="skip">{{ past ? 'Wasn’t charged' : 'Skip this one' }}</button>
        <button v-if="overrideId" type="button" class="btn" :disabled="busy" @click="backToUsual">
          <Undo2 :size="16" /> Back to the usual
        </button>
      </div>
    </form>
    <CurrencySheet v-if="pickingCurrency" v-model="currency" @close="pickingCurrency = false" />
    <DateSheet v-if="pickingDay" v-model="day" title="Charged on" :min="today" @close="pickingDay = false" />
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
.row {
  display: flex;
  gap: 8px;
}
.row .btn {
  flex: 1;
}
.hint,
.error {
  margin: 0;
}
</style>
