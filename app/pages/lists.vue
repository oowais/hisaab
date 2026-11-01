<script setup lang="ts">
// Categories and payment methods: rename in place, a category's colour
// (Stats uses it), order, and delete with undo (the payments in it keep
// everything else, and get it back on undo). Adding works here and in
// Quick add's More.
import { Check, CreditCard, MoreVertical, Plus, Tag } from 'lucide-vue-next'
import type { Category, PaymentMethod } from '~/composables/api'

const toast = useToast()
const categories = ref<Category[]>([])
const methods = ref<PaymentMethod[]>([])
const counts = ref<{ cat: Map<string, number>; method: Map<string, number> }>({ cat: new Map(), method: new Map() })
const error = ref('')
const loaded = ref(false)

async function load() {
  try {
    const [c, m, o] = await Promise.all([api.categories(), api.paymentMethods(), api.overview(todayIso())])
    categories.value = c
    methods.value = m
    const cat = new Map<string, number>()
    const method = new Map<string, number>()
    for (const s of o.subscriptions) {
      if (s.categoryId) cat.set(s.categoryId, (cat.get(s.categoryId) ?? 0) + 1)
      if (s.paymentMethodId) method.set(s.paymentMethodId, (method.get(s.paymentMethodId) ?? 0) + 1)
    }
    counts.value = { cat, method }
    error.value = ''
  } catch (e) {
    error.value = friendlyError(e, 'load the lists')
  } finally {
    loaded.value = true
  }
}
onMounted(load)
watch(dataVersion, load)

const used = (n: number | undefined) => (n ? `${n} ${n === 1 ? 'payment' : 'payments'}` : 'Not used')

async function run(f: () => Promise<unknown>, doing: string) {
  try {
    await f()
    changed()
  } catch (e) {
    error.value = friendlyError(e, doing)
    await load()
  }
}

// ---- Categories
const colorOf = (c: Category, i: number) => c.color || PALETTE[i % 8]!
const pickingColor = ref<string | null>(null)
function rename(c: Category, name: string) {
  if (!name.trim() || name.trim() === c.name) return
  run(() => api.updateCategory({ ...c, name: name.trim() }), 'rename it')
}
function setColor(c: Category, color: string) {
  pickingColor.value = null
  run(() => api.updateCategory({ ...c, color }), 'change the colour')
}
function move(i: number, by: number) {
  const ids = categories.value.map((c) => c.id)
  const [id] = ids.splice(i, 1)
  ids.splice(i + by, 0, id!)
  run(() => api.sortCategories(ids), 'move it')
}
async function removeCategory(c: Category) {
  try {
    const subs = await api.deleteCategory(c.id)
    changed()
    toast.show({
      message: `${c.name} deleted`,
      action: { label: 'Undo', run: () => run(() => api.restoreCategory(c.id, subs), 'undo it') },
    })
  } catch (e) {
    error.value = friendlyError(e, 'delete it')
  }
}
function categoryMenu(e: MouseEvent, c: Category, i: number) {
  openMenu(e, [
    ...(i > 0 ? [{ label: 'Move up', run: () => move(i, -1) }] : []),
    ...(i < categories.value.length - 1 ? [{ label: 'Move down', run: () => move(i, 1) }] : []),
    { label: 'Colour', run: () => (pickingColor.value = c.id) },
    'separator',
    { label: 'Delete', danger: true, run: () => removeCategory(c) },
  ])
}
const newCategory = ref('')
function addCategory() {
  const n = newCategory.value.trim()
  if (!n) return
  newCategory.value = ''
  run(() => api.addCategory(n), 'add the category')
}

// ---- Payment methods
function renameMethod(m: PaymentMethod, name: string) {
  if (!name.trim() || name.trim() === m.name) return
  run(() => api.renamePaymentMethod(m.id, name.trim()), 'rename it')
}
async function removeMethod(m: PaymentMethod) {
  try {
    const subs = await api.deletePaymentMethod(m.id)
    changed()
    toast.show({
      message: `${m.name} deleted`,
      action: { label: 'Undo', run: () => run(() => api.restorePaymentMethod(m.id, subs), 'undo it') },
    })
  } catch (e) {
    error.value = friendlyError(e, 'delete it')
  }
}
const newMethod = ref('')
function addMethod() {
  const n = newMethod.value.trim()
  if (!n) return
  newMethod.value = ''
  run(() => api.addPaymentMethod(n), 'add the payment method')
}
const blurOnEnter = (e: KeyboardEvent) => (e.target as HTMLInputElement).blur()
</script>

<template>
  <Screen title="Categories &amp; methods">
    <p v-if="error" class="error">{{ error }}</p>
    <template v-if="loaded">
      <h2 class="section-title">Categories</h2>
      <ul class="pane rows list">
        <li v-for="(c, i) in categories" :key="c.id" class="item" @contextmenu="categoryMenu($event, c, i)">
          <div class="row">
            <button
              class="swatch"
              :style="{ background: colorOf(c, i) }"
              :aria-label="`Colour of ${c.name}`"
              @click="pickingColor = pickingColor === c.id ? null : c.id"
            />
            <span class="flex stack">
              <input
                class="name"
                :value="c.name"
                :aria-label="`Name of ${c.name}`"
                enterkeyhint="done"
                @change="rename(c, ($event.target as HTMLInputElement).value)"
                @keydown.enter="blurOnEnter"
              />
              <span class="muted small">{{ used(counts.cat.get(c.id)) }}</span>
            </span>
            <button class="icon-btn" :aria-label="`More for ${c.name}`" @click="categoryMenu($event, c, i)">
              <MoreVertical :size="18" />
            </button>
          </div>
          <div v-if="pickingColor === c.id" class="swatches" role="radiogroup" :aria-label="`Colour of ${c.name}`">
            <button
              v-for="p in PALETTE"
              :key="p"
              role="radio"
              class="swatch big"
              :aria-checked="colorOf(c, i).toLowerCase() === p"
              :style="{ background: p, color: glyphOn(p) }"
              @click="setColor(c, p)"
            >
              <Check v-if="colorOf(c, i).toLowerCase() === p" :size="14" :stroke-width="3" />
            </button>
          </div>
        </li>
        <li class="item">
          <form class="row" @submit.prevent="addCategory">
            <Tag :size="18" class="muted lead" />
            <input v-model="newCategory" class="name flex" placeholder="New category" enterkeyhint="done" />
            <button v-if="newCategory.trim()" class="icon-btn" aria-label="Add category"><Plus :size="18" /></button>
          </form>
        </li>
      </ul>

      <h2 class="section-title">Payment methods</h2>
      <ul class="pane rows list">
        <li
          v-for="m in methods"
          :key="m.id"
          class="item"
          @contextmenu="openMenu($event, [{ label: 'Delete', danger: true, run: () => removeMethod(m) }])"
        >
          <div class="row">
            <CreditCard :size="18" class="muted lead" />
            <span class="flex stack">
              <input
                class="name"
                :value="m.name"
                :aria-label="`Name of ${m.name}`"
                enterkeyhint="done"
                @change="renameMethod(m, ($event.target as HTMLInputElement).value)"
                @keydown.enter="blurOnEnter"
              />
              <span class="muted small">{{ used(counts.method.get(m.id)) }}</span>
            </span>
            <button
              class="icon-btn"
              :aria-label="`More for ${m.name}`"
              @click="openMenu($event, [{ label: 'Delete', danger: true, run: () => removeMethod(m) }])"
            >
              <MoreVertical :size="18" />
            </button>
          </div>
        </li>
        <li class="item">
          <form class="row" @submit.prevent="addMethod">
            <Plus :size="18" class="muted lead" />
            <input v-model="newMethod" class="name flex" placeholder="New payment method, e.g. Visa ..4417" enterkeyhint="done" />
            <button v-if="newMethod.trim()" class="icon-btn" aria-label="Add payment method"><Check :size="18" /></button>
          </form>
        </li>
      </ul>
    </template>
  </Screen>
</template>

<style scoped>
.list {
  margin: 0;
}
.item {
  list-style: none;
  display: flex;
  flex-direction: column;
}
.row {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 56px;
}
.flex {
  flex: 1;
  min-width: 0;
}
.stack {
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.name {
  background: none;
  border: none;
  outline: none;
  padding: 2px 0;
  font: inherit;
  color: var(--text);
  min-width: 0;
  width: 100%;
}
.name:focus {
  box-shadow: 0 1px 0 var(--text);
}
.lead {
  width: 22px;
  flex: none;
}
.swatch {
  width: 22px;
  height: 22px;
  border-radius: 7px;
  flex: none;
  display: grid;
  place-items: center;
}
.swatch.big {
  width: 32px;
  height: 32px;
  border-radius: 10px;
}
.swatches {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 0 0 12px 34px;
}
</style>
