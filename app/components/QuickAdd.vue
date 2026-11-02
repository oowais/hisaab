<script setup lang="ts">
// Quick add: search the templates (or type any name), then the amount and a
// sentence to tap: "monthly, starting today". Below, what it is as grouped
// chips (PaymentDetails); rarer things behind "+" chips.
// From "Share → Hisaab" it opens on step 2 with what the shared text gave.
// Duplicate (`copy`): step 2 with everything of that payment; "Include
// history" copies its periods, changed and one-time charges as they are.
import { Plus, X } from 'lucide-vue-next'
import type { Detail, Kind, SubscriptionFields, Unit } from '~/composables/api'
import { amountText as amountTextOf } from '~/utils/money'
import type { Shared } from '~/utils/share'
import { TEMPLATES, type Template } from '~/utils/templates'

const props = defineProps<{ share?: Shared | null; copy?: Detail | null }>()
const emit = defineEmits<{ close: []; added: [id: string] }>()
useBackClose(() => emit('close'))
const drag = useSheetDrag(() => emit('close'))
const toast = useToast()
const today = todayIso()

// ---- Step 1: what
const query = ref('')
const template = ref<Template | null>(null)
const name = ref('')
const chosen = computed(() => !!name.value.trim())
const matches = computed(() => searchTemplates(query.value, 6))
const search = ref<HTMLInputElement>()
onMounted(() => {
  if (!props.share && !props.copy) search.value?.focus()
})

// ---- Step 2: how much, how often, from when
const amountText = ref('')
const currency = ref('EUR')
const everyN = ref(1)
const unit = ref<Unit>('month')
const start = ref(today)
const kind = ref<Kind>('subscription')
const amountInput = ref<HTMLInputElement>()

onMounted(async () => {
  const s = await api.settings().catch(() => null)
  if (s && !template.value?.currency && !props.share?.amount) currency.value = s.lastCurrency ?? s.mainCurrency
})

// The tile: the template's (or a random colour), until changed in the icon sheet.
const icon = ref<string | null>(null)
const color = ref<string | null>(null)
const newColor = randomColor()
const pickingIcon = ref(false)

function choose(t: Template | null) {
  template.value = t
  icon.value = t?.icon ?? null
  color.value = t?.color ?? newColor
  name.value = t?.name ?? query.value.trim()
  if (t) {
    kind.value = t.kind
    everyN.value = t.every ?? 1
    unit.value = t.unit ?? 'month'
    if (t.currency) currency.value = t.currency
    if (t.cancelUrl && !url.value.trim()) url.value = t.cancelUrl
  }
  nextTick(() => amountInput.value?.focus())
}
function back() {
  template.value = null
  name.value = ''
  nextTick(() => search.value?.focus())
}

const pickingInterval = ref(false)
const pickingStart = ref(false)
const pickingCurrency = ref(false)
const INTERVALS: { n: number; unit: Unit }[] = [
  { n: 1, unit: 'week' },
  { n: 1, unit: 'month' },
  { n: 3, unit: 'month' },
  { n: 6, unit: 'month' },
  { n: 1, unit: 'year' },
]
const custom = ref(false)
const startLabel = computed(() => (start.value === today ? 'today' : relativeDay(start.value)))

// Shared: the percent that's yours (null = all).
const myShare = ref<number | null>(null)

// Monthly budget (Settings): say so when this one takes you over it.
const { data: overview } = useOverview()
const addsMonthly = ref<number | null>(null)
let budgetTimer: ReturnType<typeof setTimeout> | undefined
watch([amountText, currency, everyN, unit], () => {
  clearTimeout(budgetTimer)
  addsMonthly.value = null
  const amount = parseAmount(amountText.value, currency.value)
  if (!overview.value?.monthlyBudget || !amount || everyN.value < 1) return
  budgetTimer = setTimeout(async () => {
    addsMonthly.value = await api.monthlyInMain(amount, currency.value, everyN.value, unit.value).catch(() => null)
  }, 250)
})
const overBudget = computed(() => {
  const o = overview.value
  if (!o?.monthlyBudget || addsMonthly.value == null) return ''
  const mine = myShare.value == null ? addsMonthly.value : Math.round((addsMonthly.value * myShare.value) / 100)
  const after = Math.round(o.yearlyTotal / 12) + mine
  if (after <= o.monthlyBudget) return ''
  const money = (n: number) => formatMoney(n, o.mainCurrency, { compact: true })
  return `This takes you to ${money(after)} a month, ${money(after - o.monthlyBudget)} over your budget.`
})

// ---- Details
const categoryId = ref<string | null>(null)
const methodId = ref<string | null>(null)
const planLabel = ref('')
const trial = ref(false)
const planOpen = ref(false)
const planInput = ref<HTMLInputElement>()
function openPlan() {
  planOpen.value = true
  nextTick(() => planInput.value?.focus())
}
const trialEnd = ref<string | null>(null)
const pickingTrialEnd = ref(false)
const remindDays = ref('')
const url = ref('')
const notes = ref('')

if (props.share) {
  const { template: t, name: n, url: link, amount: a } = props.share
  if (link) url.value = link
  query.value = n
  choose(t)
  if (a) {
    currency.value = a.currency
    amountText.value = amountTextOf(a.minor, a.currency)
  }
}
// ---- Duplicate
const includeHistory = ref(false)
if (props.copy) {
  const c = props.copy
  const p = c.current ?? c.periods.at(-1) ?? null
  template.value = TEMPLATES.find((t) => t.key === c.templateKey) ?? null
  name.value = `${c.name} (copy)`
  icon.value = c.icon
  color.value = c.color
  kind.value = c.kind
  categoryId.value = c.categoryId
  methodId.value = c.paymentMethodId
  myShare.value = c.myShare
  remindDays.value = c.remindDaysBefore == null ? '' : String(c.remindDaysBefore)
  url.value = c.url ?? ''
  notes.value = c.notes
  if (p) {
    currency.value = p.currency
    amountText.value = p.amountMinor ? amountTextOf(p.amountMinor, p.currency) : ''
    everyN.value = p.everyN
    unit.value = p.unit
    planLabel.value = p.planLabel ?? ''
  }
}
const historyText = computed(() => {
  const c = props.copy
  if (!c) return ''
  const n = c.periods.length
  return `${n} ${n === 1 ? 'period' : 'periods'} and every charge as it was`
})

watch(trial, (on) => {
  if (on && !trialEnd.value) trialEnd.value = addDays(start.value, 13)
})
// ---- Save
const error = ref('')
const saving = ref(false)
const amount = computed(() => parseAmount(amountText.value, currency.value))
async function fields(): Promise<SubscriptionFields> {
  const days = remindDays.value.trim() === '' ? null : Number(remindDays.value)
  return {
    kind: kind.value,
    name: name.value.trim(),
    templateKey: template.value?.key ?? null,
    icon: icon.value,
    color: color.value,
    categoryId: await savedCategoryId(categoryId.value),
    paymentMethodId: methodId.value,
    url: url.value.trim() || null,
    notes: notes.value.trim(),
    remindDaysBefore: days != null && Number.isInteger(days) && days >= 0 ? days : null,
    myShare: myShare.value,
  }
}

async function save() {
  if (!chosen.value) return
  const withHistory = !!props.copy && includeHistory.value
  if (!withHistory && amount.value == null) {
    error.value = 'Enter the amount.'
    return amountInput.value?.focus()
  }
  saving.value = true
  try {
    const id = withHistory
      ? await api.duplicateSubscription(props.copy!.id, await fields())
      : await api.addSubscription({
          fields: await fields(),
          terms: {
            amountMinor: amount.value!,
            currency: currency.value,
            everyN: everyN.value,
            unit: unit.value,
            planLabel: planLabel.value.trim() || null,
            amountKind: 'fixed',
          },
          startDate: start.value,
          anchorDate: null,
          trialEnd: trial.value ? trialEnd.value : null,
        })
    changed()
    toast.show({ message: `Added ${name.value.trim()}` })
    emit('added', id)
    emit('close')
  } catch (e) {
    error.value = friendlyError(e, 'add it')
  } finally {
    saving.value = false
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
      @submit.prevent="chosen ? save() : choose(matches[0] ?? null)"
    >
      <!-- Step 1: search -->
      <template v-if="!chosen">
        <input
          ref="search"
          :value="query"
          @input="query = ($event.target as HTMLInputElement).value"
          class="main-input"
          placeholder="Netflix, rent, gym…"
          enterkeyhint="next"
          autocapitalize="words"
        />
        <ul class="matches">
          <li v-for="t in matches" :key="t.key">
            <button type="button" class="match" @click="choose(t)">
              <SubIcon :icon="t.icon" :color="t.color" :name="t.name" :size="32" />
              <span>{{ t.name }}</span>
            </button>
          </li>
          <li v-if="query.trim() && !matches.some((t) => t.name.toLowerCase() === query.trim().toLowerCase())">
            <button type="button" class="match" @click="choose(null)">
              <SubIcon :icon="null" :color="newColor" :name="query" :size="32" />
              <span>Add “{{ query.trim() }}”</span>
            </button>
          </li>
        </ul>
      </template>

      <!-- Step 2: details -->
      <template v-else>
        <header class="head">
          <button type="button" class="tile-btn" aria-label="Change icon and colour" @click="pickingIcon = true">
            <SubIcon :icon="icon" :color="color" :name="name" :size="36" />
          </button>
          <input v-model="name" class="name-input" aria-label="Name" />
          <button type="button" class="icon-btn" aria-label="Pick something else" @click="back"><X :size="20" /></button>
        </header>

        <div v-if="copy" class="history">
          <button type="button" class="chip" :class="includeHistory ? 'chosen' : 'off'" @click="includeHistory = !includeHistory">
            <Plus v-if="!includeHistory" :size="14" /> Include history
          </button>
          <span class="muted small">{{ includeHistory ? `Copies ${historyText}` : 'Starts fresh from the day you pick' }}</span>
        </div>

        <div v-if="!includeHistory" class="amount-row">
          <input
            ref="amountInput"
            v-model="amountText"
            class="field amount"
            inputmode="decimal"
            placeholder="0.00"
            aria-label="Amount"
            enterkeyhint="done"
          />
          <button type="button" class="field currency" aria-label="Currency" @click="pickingCurrency = true">{{ currency }}</button>
        </div>

        <p v-if="overBudget && !includeHistory" class="over small">{{ overBudget }}</p>
        <p v-if="!includeHistory" class="sentence">
          <button type="button" class="word" :aria-expanded="pickingInterval" @click="pickingInterval = !pickingInterval">
            {{ intervalText(everyN, unit) }}</button
          >, {{ trial ? 'trial' : 'starting' }}
          <button type="button" class="word" @click="pickingStart = true">{{ startLabel }}</button>
        </p>
        <div v-if="pickingInterval" class="chips">
          <button
            v-for="i in INTERVALS"
            :key="`${i.n}${i.unit}`"
            type="button"
            class="chip"
            :class="{ chosen: !custom && i.n === everyN && i.unit === unit }"
            @click="((everyN = i.n), (unit = i.unit), (custom = false), (pickingInterval = false))"
          >
            {{ intervalText(i.n, i.unit) }}
          </button>
          <button type="button" class="chip" :class="{ chosen: custom }" @click="custom = true">Other…</button>
        </div>
        <div v-if="pickingInterval && custom" class="custom">
          <span>every</span>
          <input v-model.number="everyN" class="field n" type="number" inputmode="numeric" min="1" max="99" aria-label="Every" />
          <button
            v-for="u in ['day', 'week', 'month', 'year'] as const"
            :key="u"
            type="button"
            class="chip"
            :class="{ chosen: unit === u }"
            @click="unit = u"
          >
            {{ u }}s
          </button>
        </div>

        <PaymentDetails
          v-model:kind="kind"
          v-model:category-id="categoryId"
          v-model:method-id="methodId"
          v-model:my-share="myShare"
          v-model:remind-days="remindDays"
          v-model:url="url"
          v-model:notes="notes"
          class="details"
          :template-key="template?.key"
        >
          <template v-if="!includeHistory" #extras>
            <button type="button" class="chip" :class="trial ? 'chosen' : 'off'" @click="trial = !trial">
              <Plus v-if="!trial" :size="14" /> Free trial
            </button>
            <button v-if="!planOpen && !planLabel" type="button" class="chip off" @click="openPlan"><Plus :size="14" /> Plan</button>
          </template>
          <template v-if="!includeHistory" #fields>
            <p v-if="trial" class="trial muted">
              Free trial, ends
              <button type="button" class="word" @click="pickingTrialEnd = true">{{ trialEnd ? relativeDay(trialEnd) : '…' }}</button>
            </p>
            <input
              v-if="planOpen || planLabel"
              ref="planInput"
              v-model="planLabel"
              class="field"
              placeholder="Plan (Standard, Family…)"
              aria-label="Plan"
            />
          </template>
        </PaymentDetails>

        <p v-if="error" class="error">{{ error }}</p>
        <button type="submit" class="btn primary" :disabled="saving">Add {{ name.trim() }}</button>
      </template>
    </form>

    <CurrencySheet v-if="pickingCurrency" v-model="currency" @close="pickingCurrency = false" />
    <IconSheet
      v-if="pickingIcon"
      :icon="icon"
      :color="color"
      :name="name"
      @pick="(i, c) => ((icon = i), (color = c))"
      @close="pickingIcon = false"
    />
    <DateSheet v-if="pickingStart" v-model="start" :title="trial ? 'Trial starts' : 'First charge'" @close="pickingStart = false" />
    <DateSheet v-if="pickingTrialEnd" v-model="trialEnd" title="Trial ends" :min="start" @close="pickingTrialEnd = false" />
  </div>
</template>

<style scoped>
.over {
  margin: 6px 0 0;
  color: var(--orange);
}
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: oklch(0 0 0 / 18%);
  display: flex;
  align-items: flex-end;
  animation: fade-in 0.15s;
}
.sheet {
  width: calc(100% - 16px);
  max-width: 560px;
  margin: 0 auto 8px;
  border-radius: 22px;
  padding: 14px 14px calc(14px + env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-height: 92vh;
  overflow-y: auto;
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
}
.main-input,
.name-input {
  border: 0;
  outline: none;
  background: none;
  font: inherit;
  font-size: 1.2rem;
  padding: 6px 2px;
  width: 100%;
  color: var(--text);
}
.name-input {
  font-weight: 600;
  flex: 1;
  min-width: 0;
}
.matches {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
}
.match {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 7px 4px;
  border-radius: 10px;
  text-align: left;
}
.match:hover {
  background: var(--glass-state);
}
.head {
  display: flex;
  align-items: center;
  gap: 10px;
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
.sentence {
  margin: 0;
  font-size: 1.05rem;
}
.word {
  font: inherit;
  font-weight: 600;
  text-decoration: underline;
  text-decoration-style: dotted;
  text-underline-offset: 4px;
  padding: 0;
  color: var(--text);
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.custom {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.custom .n {
  width: 4.5em;
  text-align: center;
}
.history {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.details {
  margin-top: 8px;
}
.trial {
  margin: 0;
}
.error {
  margin: 0;
}
.tile-btn {
  padding: 0;
  display: inline-flex;
}
</style>
