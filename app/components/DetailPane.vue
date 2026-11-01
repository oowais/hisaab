<script setup lang="ts">
// Detail's frame in the wide-window split (Screen's twin without the fixed
// page): title, close, the #actions; the body scrolls on its own.
import { X } from 'lucide-vue-next'
defineProps<{ title: string }>()
const emit = defineEmits<{ close: [] }>()
</script>

<template>
  <section class="detail-pane">
    <header class="topbar">
      <h1>{{ title }}</h1>
      <slot name="actions" />
      <button class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
    </header>
    <div class="body"><slot /></div>
  </section>
</template>

<style scoped>
.detail-pane {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.topbar {
  flex: none;
}
.body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 0 4px 48px;
}
</style>
