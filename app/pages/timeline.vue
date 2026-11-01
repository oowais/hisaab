<script setup lang="ts">
// Timeline: one lane per subscription over time. Built in milestone 4.
import { ChartGantt } from 'lucide-vue-next'
</script>

<template>
  <main class="page">
    <header class="topbar">
      <h1>Timeline</h1>
    </header>
    <div class="empty">
      <ChartGantt :size="40" class="muted" />
      <p>Nothing to show yet</p>
      <p class="muted small">Each subscription gets a lane: when it ran, paused and changed price.</p>
    </div>
  </main>
</template>

<style scoped>
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
