<script setup lang="ts">
// Currency picker: common ones first, then every ISO 4217 code, filterable.
import { X } from 'lucide-vue-next'

const props = defineProps<{ modelValue: string }>()
const emit = defineEmits<{ 'update:modelValue': [code: string]; close: [] }>()
useBackClose(() => emit('close'))
const drag = useSheetDrag(() => emit('close'))

const query = ref('')
const all = (() => {
  try {
    return Intl.supportedValuesOf('currency')
  } catch {
    return COMMON_CURRENCIES
  }
})()
const names = new Intl.DisplayNames(undefined, { type: 'currency' })
const label = (c: string) => names.of(c) ?? c
const list = computed(() => {
  const q = query.value.trim().toLowerCase()
  const ordered = [...COMMON_CURRENCIES.filter((c) => all.includes(c)), ...all.filter((c) => !COMMON_CURRENCIES.includes(c))]
  return q ? ordered.filter((c) => c.toLowerCase().includes(q) || label(c).toLowerCase().includes(q)) : ordered
})
function pick(c: string) {
  emit('update:modelValue', c)
  emit('close')
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
      aria-label="Currency"
    >
      <header class="head">
        <!-- :value + @input, not v-model: v-model waits for the Android keyboard to commit the word. -->
        <input
          :value="query"
          @input="query = ($event.target as HTMLInputElement).value"
          class="field"
          placeholder="Search currencies"
          autocapitalize="characters"
        />
        <button class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>
      <ul class="list">
        <li v-for="c in list" :key="c">
          <button class="item" :class="{ chosen: c === props.modelValue }" @click="pick(c)">
            <b>{{ c }}</b
            ><span class="muted">{{ label(c) }}</span>
          </button>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 60;
  background: oklch(0 0 0 / 22%);
  display: flex;
  align-items: flex-end;
  animation: fade-in 0.15s;
}
.sheet {
  width: calc(100% - 16px);
  max-width: 520px;
  height: 70vh;
  margin: 0 auto 8px;
  border-radius: 22px;
  padding: 12px 10px calc(10px + env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: 8px;
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
}
.head {
  display: flex;
  gap: 6px;
  align-items: center;
}
.head .field {
  flex: 1;
}
.list {
  list-style: none;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  flex: 1;
}
.item {
  display: flex;
  gap: 12px;
  align-items: baseline;
  width: 100%;
  padding: 10px 12px;
  border-radius: 10px;
  text-align: left;
}
.item b {
  width: 3.2em;
  font-variant-numeric: tabular-nums;
}
</style>
