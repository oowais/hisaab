<script setup lang="ts">
// The gear at the top right of the main screens (as in Befikr), with a dot
// when an update is ready.
import { Settings } from 'lucide-vue-next'

const update = useUpdate()
</script>

<template>
  <NuxtLink to="/settings" class="icon-btn gear" :aria-label="update.available.value ? 'Settings (update available)' : 'Settings'">
    <Settings :size="22" /><span v-if="update.available.value" class="update-dot" />
  </NuxtLink>
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
</style>
