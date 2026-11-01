<script setup lang="ts">
// A shared payment: the part that's yours. Just me · ½ · ⅓ · ¼ · other %.
// The charge stays whole; totals, Stats and the budget count your part.
const props = defineProps<{ modelValue: number | null }>()
const emit = defineEmits<{ 'update:modelValue': [share: number | null] }>()

const PICKS = [
  { label: 'Just me', value: null },
  { label: 'Half', value: 50 },
  { label: 'A third', value: 33 },
  { label: 'A quarter', value: 25 },
]
const custom = ref(props.modelValue != null && !PICKS.some((p) => p.value === props.modelValue))
const text = ref(custom.value ? String(props.modelValue) : '')
function pick(v: number | null) {
  custom.value = false
  emit('update:modelValue', v)
}
watch(text, (t) => {
  const n = Number(t)
  if (Number.isInteger(n) && n >= 1 && n <= 99) emit('update:modelValue', n)
})
</script>

<template>
  <div class="share">
    <div class="chips" role="radiogroup" aria-label="Your share">
      <button
        v-for="p in PICKS"
        :key="p.label"
        type="button"
        role="radio"
        class="chip"
        :aria-checked="!custom && modelValue === p.value"
        :class="{ chosen: !custom && modelValue === p.value }"
        @click="pick(p.value)"
      >
        {{ p.label }}
      </button>
      <button type="button" class="chip" :class="{ chosen: custom }" @click="custom = true">Other</button>
    </div>
    <label v-if="custom" class="custom">
      <input v-model="text" class="field" inputmode="numeric" placeholder="40" aria-label="Your share in percent" />
      <span class="muted">% is yours</span>
    </label>
  </div>
</template>

<style scoped>
.share {
  display: flex;
  flex-direction: column;
  gap: 8px;
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
}
.custom .field {
  width: 5rem;
  text-align: right;
}
</style>
