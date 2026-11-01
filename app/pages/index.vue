<script setup lang="ts">
// Upcoming (home): the next charges, grouped by when. Filled in by milestone 3.
import { CalendarClock, Settings } from 'lucide-vue-next'

const update = useUpdate()
</script>

<template>
  <main class="page">
    <header class="topbar">
      <h1>Hisaab</h1>
      <NuxtLink to="/settings" class="icon-btn gear" :aria-label="update.available.value ? 'Settings (update available)' : 'Settings'">
        <Settings :size="22" /><span v-if="update.available.value" class="update-dot" />
      </NuxtLink>
    </header>
    <div class="empty">
      <CalendarClock :size="40" class="muted" />
      <p>No payments yet</p>
      <p class="muted small">Subscriptions and bills you add show up here, next charge first.</p>
    </div>
  </main>
</template>

<style scoped>
.gear {
  position: relative;
}
.update-dot {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent);
  box-shadow: 0 0 0 2px var(--bg);
}
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  margin-top: 30vh;
  text-align: center;
}
.empty p {
  margin: 0;
}
</style>
