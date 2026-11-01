<script setup lang="ts">
// Shared page frame: back button, title and optional #actions pinned at the
// top, the page's content scrolling below. Built like the home screen
// (index.vue): fixed between the status and gesture bars, so only the body
// ever scrolls. `tab`: a main screen (Calendar, Settings), no back button and
// room for the tab bar.
import { ArrowLeft } from 'lucide-vue-next'
defineProps<{ title: string; tab?: boolean }>()
const router = useRouter()
</script>

<template>
  <main class="page screen">
    <header class="topbar">
      <button v-if="!tab" class="icon-btn" aria-label="Back" @click="router.back()"><ArrowLeft :size="22" /></button>
      <h1>{{ title }}</h1>
      <slot name="actions" />
    </header>
    <div class="body" :class="{ tab }"><slot /></div>
  </main>
</template>

<style scoped>
.screen {
  position: fixed;
  top: env(safe-area-inset-top);
  bottom: env(safe-area-inset-bottom);
  left: 0;
  right: 0;
  display: flex;
  flex-direction: column;
  padding-bottom: 0;
}
.topbar {
  flex: none;
}
/* Full width so the scrollbar (desktop) sits at the window edge. */
.body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  margin: 0 -16px;
  padding: 0 16px 48px;
}
.body.tab {
  padding-bottom: calc(110px + env(safe-area-inset-bottom));
}
</style>
