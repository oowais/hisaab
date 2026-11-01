<script setup lang="ts">
// The status actions of a subscription, each asking only what it needs:
// price change (amount, from when, plan), billing day (the next charge's
// day; later ones follow it, earlier ones stay), pause (from, optional resume),
// resume, cancel (ends on), restart, trial → paid. Dates default to what
// usually fits (next charge, end of the running cycle, today).
import { X } from 'lucide-vue-next'
import type { Action, Detail, Terms } from '~/composables/api'

const props = defineProps<{ action: Action; sub: Detail }>()
const emit = defineEmits<{ close: []; done: [message: string] }>()
useBackClose(() => emit('close'))

const today = todayIso()
const TITLES: Record<Action, string> = {
  price: 'Change price',
  billing: 'Change billing day',
  pause: 'Pause',
  resume: 'Resume',
  cancel: 'Cancel',
  restart: 'Restart',
  convert: 'Convert to paid',
}
// Terms to start from: the current period's, or for a trial the paid period after it.
const base = computed<Terms | null>(() => {
  const ps = props.sub.periods
  if (props.action === 'convert') {
    const after = ps.find((p) => p.status === 'active' && p.startDate > today)
    if (after) return after
  }
  if (props.action === 'restart') return [...ps].reverse().find((p) => p.status !== 'trial') ?? ps.at(-1) ?? null
  return props.sub.current
})
const needsAmount = computed(() => ['price', 'restart', 'convert'].includes(props.action))

const amountText = ref(base.value ? (base.value.amountMinor ? amountTextOf(base.value) : '') : '')
const currency = ref(base.value?.currency ?? 'EUR')
const planLabel = ref(base.value?.planLabel ?? '')
function amountTextOf(t: Terms) {
  return (t.amountMinor / 10 ** minorDigits(t.currency)).toFixed(minorDigits(t.currency))
}

const day = ref(today)
const resumeOn = ref<string | null>(null)
const defaults = ref<{ nextCharge: string | null; cycleEnd: string } | null>(null)
onMounted(async () => {
  defaults.value = await api.defaults(props.sub.id, today).catch(() => null)
  if ((props.action === 'price' || props.action === 'billing') && defaults.value?.nextCharge) day.value = defaults.value.nextCharge
  if (props.action === 'cancel' && defaults.value) day.value = defaults.value.cycleEnd
})
const picks = computed(() => {
  const d = defaults.value
  const out: { label: string; day: string }[] = []
  if (props.action === 'price' && d?.nextCharge) out.push({ label: 'Next charge', day: d.nextCharge })
  if (props.action === 'cancel' && d) out.push({ label: 'End of this cycle', day: d.cycleEnd })
  return out
})
const dayLabel = computed(
  () =>
    ({
      price: 'From',
      billing: 'Next charge on',
      pause: 'Pause from',
      resume: 'First charge again',
      cancel: 'Ends on',
      restart: 'First charge',
      convert: 'First paid charge',
    })[props.action],
)
const picking = ref<'day' | 'resume' | null>(null)
const pickingCurrency = ref(false)

const error = ref('')
const busy = ref(false)
async function run() {
  error.value = ''
  let terms: Terms | null = null
  if (needsAmount.value && base.value) {
    const amount = parseAmount(amountText.value, currency.value)
    if (amount == null) return void (error.value = 'Enter the amount.')
    const b = base.value
    terms = {
      amountMinor: amount,
      currency: currency.value,
      everyN: b.everyN,
      unit: b.unit,
      planLabel: planLabel.value.trim() || null,
      amountKind: 'fixed',
    }
  }
  busy.value = true
  const id = props.sub.id
  try {
    switch (props.action) {
      case 'price':
        await api.changeTerms(id, day.value, terms!)
        break
      case 'billing':
        await api.changeBillingDay(id, day.value, today)
        break
      case 'pause':
        await api.pause(id, day.value, resumeOn.value)
        break
      case 'resume':
        await api.resume(id, day.value)
        break
      case 'cancel':
        await api.cancel(id, day.value)
        break
      case 'restart':
        await api.restart(id, day.value, terms)
        break
      case 'convert':
        await api.convertTrial(id, day.value, terms)
        break
    }
    changed()
    const when = relativeDay(day.value, today)
    const on = shortDay(day.value, today)
    emit(
      'done',
      {
        price: `New price from ${on}`,
        billing: `Billing day changed: next charge ${when}`,
        pause: `Paused from ${on}`,
        resume: `Resumes ${when}`,
        cancel: `Ends ${when}`,
        restart: `Restarts ${when}`,
        convert: `Paid from ${on}`,
      }[props.action],
    )
  } catch (e) {
    error.value = friendlyError(e, TITLES[props.action].toLowerCase())
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <form class="sheet popover" role="dialog" :aria-label="TITLES[action]" @submit.prevent="run">
      <header class="head">
        <b>{{ TITLES[action] }} · {{ sub.name }}</b>
        <button type="button" class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>

      <div v-if="needsAmount" class="amount-row">
        <input v-model="amountText" class="field amount" inputmode="decimal" placeholder="0.00" aria-label="Amount" />
        <button type="button" class="field currency" aria-label="Currency" @click="pickingCurrency = true">{{ currency }}</button>
      </div>
      <input v-if="action === 'price'" v-model="planLabel" class="field" placeholder="Plan (optional)" aria-label="Plan" />

      <button type="button" class="pick" @click="picking = 'day'">
        <span class="muted small">{{ dayLabel }}</span>
        <span
          >{{ shortDay(day) }} <span class="muted">· {{ relativeDay(day, today) }}</span></span
        >
      </button>
      <p v-if="action === 'billing'" class="muted small hint">Charges from this day on follow it. Charges before it stay as they were.</p>
      <button v-if="action === 'pause'" type="button" class="pick" @click="picking = 'resume'">
        <span class="muted small">Resume</span>
        <span>{{ resumeOn ? shortDay(resumeOn) : 'When I say so' }}</span>
      </button>

      <p v-if="error" class="error">{{ error }}</p>
      <button type="submit" class="btn primary" :class="{ danger: action === 'cancel' }" :disabled="busy">{{ TITLES[action] }}</button>
    </form>

    <DateSheet
      v-if="picking === 'day'"
      v-model="day"
      :title="dayLabel"
      :picks="picks"
      :min="action === 'billing' ? today : null"
      @close="picking = null"
    />
    <DateSheet
      v-if="picking === 'resume'"
      v-model="resumeOn"
      title="Resume on"
      :min="addDays(day, 1)"
      allow-clear
      clear-label="No date"
      @close="picking = null"
    />
    <CurrencySheet v-if="pickingCurrency" v-model="currency" @close="pickingCurrency = false" />
  </div>
</template>

<style scoped>
.hint {
  margin: -4px 4px 0;
}
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
.error {
  margin: 0;
}
</style>
