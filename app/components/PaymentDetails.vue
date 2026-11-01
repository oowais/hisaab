<script setup lang="ts">
// What a payment is, as grouped chips (Quick add and Edit): subscription or
// bill, category (yours, then pre-made ones), paid with, your part. Rarer
// things wait behind "+" chips (reminder, link, note; the parent adds its
// own in the `extras` and `fields` slots) and open once tapped or set.
import { Plus } from 'lucide-vue-next'
import type { Category, Kind, PaymentMethod } from '~/composables/api'
import { categoryChips, suggestedFor } from '~/utils/categories'

const props = defineProps<{ templateKey?: string | null }>()
const kind = defineModel<Kind>('kind', { required: true })
const categoryId = defineModel<string | null>('categoryId', { required: true })
const methodId = defineModel<string | null>('methodId', { required: true })
const myShare = defineModel<number | null>('myShare', { required: true })
const remindDays = defineModel<string>('remindDays', { required: true })
const url = defineModel<string>('url', { required: true })
const notes = defineModel<string>('notes', { required: true })

const toast = useToast()
const categories = ref<Category[]>([])
const methods = ref<PaymentMethod[]>([])
const chips = computed(() => categoryChips(categories.value))

// A template's usual category, unless one was picked (yours by that name wins).
let touched = categoryId.value != null
watch(categoryId, () => (touched = true), { flush: 'sync' })
function preselect() {
  const s = suggestedFor(props.templateKey)
  if (touched || !s) return
  const mine = categories.value.find((c) => c.id === s.id || c.name.trim().toLowerCase() === s.name.toLowerCase())
  categoryId.value = mine?.id ?? s.id
  touched = false
}
watch(() => props.templateKey, preselect)
onMounted(async () => {
  ;[categories.value, methods.value] = await Promise.all([api.categories().catch(() => []), api.paymentMethods().catch(() => [])])
  preselect()
})

async function addCategory(name: string) {
  const c = await api.addCategory(name).catch((e) => void toast.show({ message: friendlyError(e, 'add the category') }))
  if (c) categories.value.push(c)
  return c
}
async function addMethod(name: string) {
  const m = await api.addPaymentMethod(name).catch((e) => void toast.show({ message: friendlyError(e, 'add the payment method') }))
  if (m) methods.value.push(m)
  return m
}

// "+" chips: open once tapped, or when already set.
const open = reactive({ remind: remindDays.value !== '', url: !!url.value, notes: !!notes.value })
const reminderInput = ref<HTMLInputElement>()
const urlInput = ref<HTMLInputElement>()
const notesInput = ref<HTMLTextAreaElement>()
function reveal(k: 'remind' | 'url' | 'notes') {
  open[k] = true
  nextTick(() => ({ remind: reminderInput, url: urlInput, notes: notesInput })[k].value?.focus())
}
watch(url, (v) => v && (open.url = true))
</script>

<template>
  <div class="details">
    <div class="chips" role="radiogroup" aria-label="Kind">
      <button type="button" role="radio" class="chip" :class="{ chosen: kind === 'subscription' }" @click="kind = 'subscription'">
        Subscription
      </button>
      <button type="button" role="radio" class="chip" :class="{ chosen: kind === 'bill' }" @click="kind = 'bill'">Bill</button>
    </div>

    <span class="label muted small">Category</span>
    <ChoiceChips v-model="categoryId" :items="chips" label="Category" placeholder="Pets, Kids…" :add="addCategory" />

    <span class="label muted small">Paid with</span>
    <ChoiceChips v-model="methodId" :items="methods" label="Payment method" placeholder="Visa ..4417, PayPal" :add="addMethod" />

    <span class="label muted small">Your part</span>
    <ShareChips v-model="myShare" />

    <div class="chips extras">
      <slot name="extras" />
      <button v-if="!open.remind" type="button" class="chip off" @click="reveal('remind')"><Plus :size="14" /> Reminder</button>
      <button v-if="!open.url" type="button" class="chip off" @click="reveal('url')"><Plus :size="14" /> Link</button>
      <button v-if="!open.notes" type="button" class="chip off" @click="reveal('notes')"><Plus :size="14" /> Note</button>
    </div>

    <slot name="fields" />
    <label v-if="open.remind" class="remind">
      <span class="muted">Remind me</span>
      <input
        ref="reminderInput"
        v-model="remindDays"
        class="field n"
        type="number"
        inputmode="numeric"
        min="0"
        max="60"
        placeholder="3"
        aria-label="Remind days before"
      />
      <span class="muted">days before</span>
    </label>
    <input
      v-if="open.url"
      ref="urlInput"
      v-model="url"
      class="field"
      type="url"
      inputmode="url"
      placeholder="Website or cancel link"
      aria-label="Link"
    />
    <textarea v-if="open.notes" ref="notesInput" v-model="notes" class="field notes" rows="2" placeholder="Notes" aria-label="Notes" />
  </div>
</template>

<style scoped>
.details {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.label {
  margin: 6px 0 -4px;
}
.extras {
  margin-top: 6px;
}
.extras :deep(.chip.off) {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.remind {
  display: flex;
  align-items: center;
  gap: 8px;
}
.remind .n {
  width: 5em;
}
.notes {
  resize: vertical;
}
</style>
