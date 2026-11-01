<script setup lang="ts">
// Pick one of a list as chips (tap again: none), or "+ New" to type and add
// one (Enter or the check). Used for categories (with the pre-made ones) and
// payment methods.
import { Check, Plus } from 'lucide-vue-next'

type Item = { id: string; name: string }
const props = defineProps<{
  modelValue: string | null
  items: Item[]
  label: string
  placeholder: string
  /** Makes the new item; it is picked once made. */
  add: (name: string) => Promise<Item | undefined>
}>()
const emit = defineEmits<{ 'update:modelValue': [id: string | null] }>()

const adding = ref(false)
const text = ref('')
const input = ref<HTMLInputElement>()
function open() {
  adding.value = true
  nextTick(() => input.value?.focus())
}
async function save() {
  const name = text.value.trim()
  if (!name) return void (adding.value = false)
  const made = await props.add(name)
  if (!made) return
  emit('update:modelValue', made.id)
  text.value = ''
  adding.value = false
}
</script>

<template>
  <div class="chips" role="radiogroup" :aria-label="label">
    <button
      v-for="i in items"
      :key="i.id"
      type="button"
      role="radio"
      class="chip"
      :aria-checked="modelValue === i.id"
      :class="{ chosen: modelValue === i.id }"
      @click="emit('update:modelValue', modelValue === i.id ? null : i.id)"
    >
      {{ i.name }}
    </button>
    <span v-if="adding" class="new">
      <input
        ref="input"
        v-model="text"
        class="field"
        :placeholder="placeholder"
        :aria-label="`New ${label.toLowerCase()}`"
        @keydown.enter.prevent="save"
        @keydown.esc.stop="adding = false"
        @blur="!text.trim() && (adding = false)"
      />
      <button v-if="text.trim()" type="button" class="icon-btn" :aria-label="`Add ${text.trim()}`" @mousedown.prevent @click="save">
        <Check :size="18" />
      </button>
    </span>
    <button v-else type="button" class="chip off add" @click="open"><Plus :size="14" /> New</button>
  </div>
</template>

<style scoped>
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.add {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.new {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.new .field {
  width: 180px;
  min-height: 36px;
  padding: 6px 12px;
}
</style>
