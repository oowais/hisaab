<script setup lang="ts">
// Payments that are over: ended ones (hidden from Upcoming) and archived
// ones (hidden everywhere else). Tap one for its history; archive or bring
// back from here.
import { Archive, ArchiveRestore } from 'lucide-vue-next'
import type { SubscriptionView } from '~/composables/api'

const toast = useToast()
const today = todayIso()
const ended = ref<SubscriptionView[]>([])
const archived = ref<SubscriptionView[] | null>(null)
const error = ref('')

async function load() {
  try {
    const [o, a] = await Promise.all([api.overview(today), api.archived(today)])
    ended.value = o.subscriptions.filter((s) => s.status === 'ended')
    archived.value = a
  } catch (e) {
    error.value = friendlyError(e, 'load the archive')
  }
}
watch(dataVersion, load)
onMounted(load)

async function setArchived(s: SubscriptionView, on: boolean) {
  try {
    await api.setArchived(s.id, on)
    changed()
    toast.show({
      message: on ? `${s.name} archived` : `${s.name} is back`,
      action: { label: 'Undo', run: () => void api.setArchived(s.id, !on).then(changed) },
    })
  } catch (e) {
    toast.show({ message: friendlyError(e, on ? 'archive it' : 'bring it back') })
  }
}

/** "Ended 3 Mar 2025", or the status for an archived one still running. */
function when(s: SubscriptionView) {
  if (s.status !== 'ended') return { trial: 'Trial', active: 'Active', paused: 'Paused', ended: '' }[s.status]
  return s.current?.endDate ? `Ended ${shortDay(s.current.endDate, today)}` : 'Ended'
}
</script>

<template>
  <Screen title="Ended and archived">
    <p v-if="error" class="error">{{ error }}</p>
    <template v-if="archived">
      <p v-if="!ended.length && !archived.length" class="muted small intro">
        Nothing here yet. Cancelled payments show up here once they end; archive them to tidy the Timeline.
      </p>

      <template v-if="ended.length">
        <h2 class="section-title">Ended</h2>
        <ul class="pane rows list">
          <li v-for="s in ended" :key="s.id" class="item">
            <NuxtLink :to="`/sub/${s.id}`" class="sub-row">
              <SubIcon :icon="s.icon" :color="s.color" :name="s.name" :size="38" />
              <span class="flex stack">
                <span>{{ s.name }}</span>
                <span class="muted small">{{ when(s) }}</span>
              </span>
            </NuxtLink>
            <button class="icon-btn" :aria-label="`Archive ${s.name}`" @click="setArchived(s, true)"><Archive :size="18" /></button>
          </li>
        </ul>
      </template>

      <template v-if="archived.length">
        <h2 class="section-title">Archived</h2>
        <ul class="pane rows list">
          <li v-for="s in archived" :key="s.id" class="item">
            <NuxtLink :to="`/sub/${s.id}`" class="sub-row">
              <SubIcon :icon="s.icon" :color="s.color" :name="s.name" :size="38" />
              <span class="flex stack">
                <span>{{ s.name }}</span>
                <span class="muted small">{{ when(s) }}</span>
              </span>
            </NuxtLink>
            <button class="icon-btn" :aria-label="`Bring back ${s.name}`" @click="setArchived(s, false)">
              <ArchiveRestore :size="18" />
            </button>
          </li>
        </ul>
      </template>
    </template>
  </Screen>
</template>

<style scoped>
.intro {
  margin: 4px 0 12px;
}
.list {
  margin: 0;
}
.item {
  display: flex;
  align-items: center;
  gap: 8px;
  list-style: none;
}
.sub-row {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 56px;
  color: inherit;
  text-decoration: none;
}
.flex {
  flex: 1;
  min-width: 0;
}
.stack {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
</style>
