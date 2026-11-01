<script setup lang="ts">
// Main screens. Phone: a bar at the bottom; wide windows: a rail on the left.
// Hidden on pages that aren't tabs.
import { CalendarClock, ChartColumn, ChartGantt } from 'lucide-vue-next'

const TABS = [
  { to: '/', label: 'Upcoming', icon: CalendarClock },
  { to: '/timeline', label: 'Timeline', icon: ChartGantt },
  { to: '/stats', label: 'Stats', icon: ChartColumn },
]
const route = useRoute()
const shown = computed(() => TABS.some((t) => t.to === route.path.replace(/(.)\/$/, '$1')))
</script>

<template>
  <nav v-if="shown" class="tabs popover" aria-label="Main">
    <NuxtLink v-for="t in TABS" :key="t.to" :to="t.to" class="tab" :class="{ chosen: route.path === t.to }">
      <component :is="t.icon" :size="22" />
      <span>{{ t.label }}</span>
    </NuxtLink>
  </nav>
</template>

<style scoped>
.tabs {
  position: fixed;
  z-index: 40;
  left: 50%;
  transform: translateX(-50%);
  bottom: calc(12px + env(safe-area-inset-bottom));
  display: flex;
  gap: 4px;
  padding: 5px;
  border-radius: 999px;
}
.tab {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  min-width: 88px;
  padding: 6px 14px;
  border-radius: 999px;
  color: var(--muted);
  text-decoration: none;
  font-size: 0.75rem;
}
.tab.chosen {
  color: var(--text);
  background: var(--pane);
  box-shadow:
    inset 0 1px 0 var(--glass-control-rim),
    inset 0 0 0 1px var(--line);
}
@media (min-width: 900px) {
  .tabs {
    left: 16px;
    top: 50%;
    bottom: auto;
    transform: translateY(-50%);
    flex-direction: column;
    border-radius: 22px;
  }
  .tab {
    min-width: 72px;
    border-radius: 16px;
  }
}
</style>
