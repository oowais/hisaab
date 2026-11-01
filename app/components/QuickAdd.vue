<script setup lang="ts">
// Quick add: search the templates (or type any name), then the amount and a
// sentence to tap: "monthly, starting today". "More" has everything else.
import { ChevronDown, ChevronUp, Plus, X } from 'lucide-vue-next'
import type { Category, Kind, PaymentMethod, Unit } from '~/composables/api'
import type { Template } from '~/utils/templates'

const emit = defineEmits<{ close: [] }>()
useBackClose(() => emit('close'))
const toast = useToast()
const today = todayIso()

// ---- Step 1: what
const query = ref('')
const template = ref<Template | null>(null)
const name = ref('')
const chosen = computed(() => !!name.value.trim())
const matches = computed(() => searchTemplates(query.value, 6))
const search = ref<HTMLInputElement>()
onMounted(() => search.value?.focus())

// ---- Step 2: how much, how often, from when
const amountText = ref('')
const currency = ref('EUR')
const everyN = ref(1)
const unit = ref<Unit>('month')
const start = ref(today)
const kind = ref<Kind>('subscription')
const estimated = ref(false)
const amountInput = ref<HTMLInputElement>()

onMounted(async () => {
  const s = await api.settings().catch(() => null)
  if (s && !template.value?.currency) currency.value = s.lastCurrency ?? s.mainCurrency
})

function choose(t: Template | null) {
  template.value = t
  name.value = t?.name ?? query.value.trim()
  if (t) {
    kind.value = t.kind
    everyN.value = t.every ?? 1
    unit.value = t.unit ?? 'month'
    estimated.value = !!t.estimated
    if (t.currency) currency.value = t.currency
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

// ---- More
const more = ref(false)
const categories = ref<Category[]>([])
const methods = ref<PaymentMethod[]>([])
const categoryId = ref<string | null>(null)
const methodId = ref<string | null>(null)
const planLabel = ref('')
const trial = ref(false)
const trialEnd = ref<string | null>(null)
const pickingTrialEnd = ref(false)
const remindDays = ref('')
const url = ref('')
const notes = ref('')
watch(more, async (open) => {
  if (!open) return
  categories.value = await api.categories().catch(() => [])
  methods.value = await api.paymentMethods().catch(() => [])
})
watch(trial, (on) => {
  if (on && !trialEnd.value) trialEnd.value = addDays(start.value, 13)
})
const newCategory = ref('')
const newMethod = ref('')
async function addCategory() {
  const n = newCategory.value.trim()
  if (!n) return
  const c = await api.addCategory(n).catch((e) => void (error.value = friendlyError(e, 'add the category')))
  if (c) {
    categories.value.push(c)
    categoryId.value = c.id
    newCategory.value = ''
  }
}
async function addMethod() {
  const n = newMethod.value.trim()
  if (!n) return
  const m = await api.addPaymentMethod(n).catch((e) => void (error.value = friendlyError(e, 'add the payment method')))
  if (m) {
    methods.value.push(m)
    methodId.value = m.id
    newMethod.value = ''
  }
}

// ---- Save
const error = ref('')
const saving = ref(false)
const amount = computed(() => parseAmount(amountText.value, currency.value))
async function save() {
  if (!chosen.value) return
  if (amount.value == null) {
    error.value = 'Enter the amount.'
    return amountInput.value?.focus()
  }
  const days = remindDays.value.trim() === '' ? null : Number(remindDays.value)
  saving.value = true
  try {
    await api.addSubscription({
      fields: {
        kind: kind.value,
        name: name.value.trim(),
        templateKey: template.value?.key ?? null,
        icon: template.value?.icon ?? null,
        color: template.value?.color ?? null,
        categoryId: categoryId.value,
        paymentMethodId: methodId.value,
        url: url.value.trim() || null,
        notes: notes.value.trim(),
        remindDaysBefore: days != null && Number.isInteger(days) && days >= 0 ? days : null,
      },
      terms: {
        amountMinor: amount.value,
        currency: currency.value,
        everyN: everyN.value,
        unit: unit.value,
        planLabel: planLabel.value.trim() || null,
        amountKind: estimated.value ? 'estimated' : 'fixed',
      },
      startDate: start.value,
      anchorDate: null,
      trialEnd: trial.value ? trialEnd.value : null,
    })
    changed()
    toast.show({ message: `Added ${name.value.trim()}` })
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
    <form class="sheet popover" @submit.prevent="chosen ? save() : choose(matches[0] ?? null)">
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
              <SubIcon :icon="null" :color="null" :name="query" :size="32" />
              <span>Add “{{ query.trim() }}”</span>
            </button>
          </li>
        </ul>
      </template>

      <!-- Step 2: details -->
      <template v-else>
        <header class="head">
          <SubIcon :icon="template?.icon ?? null" :color="template?.color ?? null" :name="name" :size="36" />
          <input v-model="name" class="name-input" aria-label="Name" />
          <button type="button" class="icon-btn" aria-label="Pick something else" @click="back"><X :size="20" /></button>
        </header>

        <div class="amount-row">
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

        <p class="sentence">
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

        <button type="button" class="btn link more-toggle" :aria-expanded="more" @click="more = !more">
          More <component :is="more ? ChevronUp : ChevronDown" :size="16" />
        </button>
        <div v-if="more" class="more">
          <div class="chips" role="radiogroup" aria-label="Kind">
            <button type="button" class="chip" :class="{ chosen: kind === 'subscription' }" @click="kind = 'subscription'">
              Subscription
            </button>
            <button type="button" class="chip" :class="{ chosen: kind === 'bill' }" @click="kind = 'bill'">Bill</button>
            <button type="button" class="chip" :class="{ chosen: estimated }" @click="estimated = !estimated">Amount varies</button>
          </div>

          <label class="label muted small">Category</label>
          <div class="chips">
            <button
              v-for="c in categories"
              :key="c.id"
              type="button"
              class="chip"
              :class="{ chosen: categoryId === c.id }"
              @click="categoryId = categoryId === c.id ? null : c.id"
            >
              {{ c.name }}
            </button>
            <span class="new">
              <input v-model="newCategory" class="field" placeholder="New category" @keydown.enter.prevent="addCategory" />
              <button v-if="newCategory.trim()" type="button" class="icon-btn" aria-label="Add category" @click="addCategory">
                <Plus :size="18" />
              </button>
            </span>
          </div>

          <label class="label muted small">Paid with</label>
          <div class="chips">
            <button
              v-for="m in methods"
              :key="m.id"
              type="button"
              class="chip"
              :class="{ chosen: methodId === m.id }"
              @click="methodId = methodId === m.id ? null : m.id"
            >
              {{ m.name }}
            </button>
            <span class="new">
              <input v-model="newMethod" class="field" placeholder="Visa ..4417, PayPal" @keydown.enter.prevent="addMethod" />
              <button v-if="newMethod.trim()" type="button" class="icon-btn" aria-label="Add payment method" @click="addMethod">
                <Plus :size="18" />
              </button>
            </span>
          </div>

          <input v-model="planLabel" class="field" placeholder="Plan (Standard, Family…)" aria-label="Plan" />

          <div class="trial">
            <button type="button" class="chip" :class="{ chosen: trial }" @click="trial = !trial">Free trial</button>
            <button v-if="trial" type="button" class="word" @click="pickingTrialEnd = true">
              ends {{ trialEnd ? relativeDay(trialEnd) : '…' }}
            </button>
          </div>

          <input
            v-model="remindDays"
            class="field"
            type="number"
            inputmode="numeric"
            min="0"
            max="60"
            placeholder="Remind days before (default)"
            aria-label="Remind days before"
          />
          <input v-model="url" class="field" type="url" inputmode="url" placeholder="Website or cancel link" aria-label="Link" />
          <textarea v-model="notes" class="field notes" rows="2" placeholder="Notes" aria-label="Notes" />
        </div>

        <p v-if="error" class="error">{{ error }}</p>
        <button type="submit" class="btn primary" :disabled="saving">Add {{ name.trim() }}</button>
      </template>
    </form>

    <CurrencySheet v-if="pickingCurrency" v-model="currency" @close="pickingCurrency = false" />
    <DateSheet v-if="pickingStart" v-model="start" :title="trial ? 'Trial starts' : 'First charge'" @close="pickingStart = false" />
    <DateSheet v-if="pickingTrialEnd" v-model="trialEnd" title="Trial ends" :min="start" @close="pickingTrialEnd = false" />
  </div>
</template>

<style scoped>
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
.more-toggle {
  align-self: flex-start;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 2px;
}
.more {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.label {
  margin: 4px 0 -4px;
}
.new {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}
.new .field {
  width: 11em;
  padding: 6px 10px;
}
.trial {
  display: flex;
  align-items: center;
  gap: 12px;
}
.notes {
  resize: vertical;
}
.error {
  margin: 0;
}
</style>
