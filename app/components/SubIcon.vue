<script setup lang="ts">
// A subscription's tile: brand glyph, generic icon or first letter, on its colour.
const props = withDefaults(defineProps<{ icon: string | null; color: string | null; name: string; size?: number }>(), { size: 40 })
const spec = computed(() => iconSpec(props.icon, props.name))
const bg = computed(() => props.color || fallbackColor(props.name))
const fg = computed(() => glyphOn(bg.value))
</script>

<template>
  <span class="tile" :style="{ width: `${size}px`, height: `${size}px`, background: bg, color: fg }" aria-hidden="true">
    <svg v-if="spec.kind === 'brand'" viewBox="0 0 24 24" :width="size * 0.55" :height="size * 0.55" fill="currentColor">
      <path :d="spec.path" />
    </svg>
    <component :is="spec.component" v-else-if="spec.kind === 'generic'" :size="size * 0.55" />
    <span v-else class="letter" :style="{ fontSize: `${size * 0.45}px` }">{{ spec.letter }}</span>
  </span>
</template>

<style scoped>
.tile {
  display: inline-grid;
  place-items: center;
  flex: none;
  border-radius: 28%;
  box-shadow:
    inset 0 1px 0 oklch(1 0 0 / 22%),
    inset 0 0 0 1px oklch(0 0 0 / 8%);
}
.letter {
  font-weight: 600;
  line-height: 1;
}
</style>
