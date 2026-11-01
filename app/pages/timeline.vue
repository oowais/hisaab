<script setup lang="ts">
// Timeline: one lane per subscription over time, with its trials, pauses
// and price changes. Tap a bar for that period in Detail, a diamond for
// what changed.
import { ChartGantt } from 'lucide-vue-next'
import type { Category, TimelineLane } from '~/composables/api'
import type { Marker } from '~/utils/timeline'

const router = useRouter()
const toast = useToast()
const today = todayIso()
const lanes = ref<TimelineLane[] | null>(null)
const categories = ref<Category[]>([])
const error = ref('')

async function load() {
  try {
    lanes.value = await api.timeline(today)
    categories.value = await api.categories().catch(() => [])
    error.value = ''
  } catch (e) {
    error.value = friendlyError(e, 'load the timeline')
  }
}
onMounted(load)
watch(dataVersion, load)

type Zoom = '6m' | '1y' | 'all'
const zoom = ref<Zoom>('1y')
type Sort = 'start' | 'cost' | 'category'
const sort = ref<Sort>('start')
const showEnded = ref(false)
const hasEnded = computed(() => !!lanes.value?.some((l) => l.status === 'ended'))

const shown = computed(() => {
  const ls = (lanes.value ?? []).filter((l) => showEnded.value || l.status !== 'ended')
  const start = (l: TimelineLane) => l.periods[0]?.startDate ?? ''
  const cat = new Map(categories.value.map((c) => [c.id, c]))
  const sorters: Record<Sort, (a: TimelineLane, b: TimelineLane) => number> = {
    start: (a, b) => start(a).localeCompare(start(b)),
    cost: (a, b) => (b.yearlyInMain ?? -1) - (a.yearlyInMain ?? -1),
    category: (a, b) => {
      const ca = a.categoryId ? cat.get(a.categoryId) : undefined
      const cb = b.categoryId ? cat.get(b.categoryId) : undefined
      // Uncategorised last; then by category order, then name.
      return (ca?.sort ?? 1e9) - (cb?.sort ?? 1e9) || a.name.localeCompare(b.name)
    },
  }
  return [...ls].sort(sorters[sort.value])
})

const open = (id: string, periodId: string) => router.push({ path: `/sub/${id}`, query: periodId ? { period: periodId } : {} })
const marker = (m: Marker) => toast.show({ message: `${shortDay(m.date)}: ${m.text}` })
</script>

<template>
  <main class="page">
    <header class="topbar">
      <h1>Timeline</h1>
    </header>
    <p v-if="error" class="error">{{ error }}</p>

    <template v-if="lanes?.length">
      <div class="controls">
        <div class="chips" role="radiogroup" aria-label="Zoom">
          <button
            v-for="z in ['6m', '1y', 'all'] as const"
            :key="z"
            class="chip"
            role="radio"
            :aria-checked="zoom === z"
            :class="{ chosen: zoom === z }"
            @click="zoom = z"
          >
            {{ { '6m': '6 months', '1y': '1 year', all: 'All' }[z] }}
          </button>
        </div>
        <div class="chips" role="radiogroup" aria-label="Sort">
          <button
            v-for="s in ['start', 'cost', 'category'] as const"
            :key="s"
            class="chip"
            role="radio"
            :aria-checked="sort === s"
            :class="{ chosen: sort === s }"
            @click="sort = s"
          >
            {{ { start: 'Start date', cost: 'Cost', category: 'Category' }[s] }}
          </button>
          <button v-if="hasEnded" class="chip" :class="{ chosen: showEnded }" :aria-pressed="showEnded" @click="showEnded = !showEnded">
            Ended
          </button>
        </div>
      </div>
      <TimelineChart v-if="shown.length" :lanes="shown" :today="today" :zoom="zoom" @open="open" @marker="marker" />
      <p v-else class="muted small quiet">Everything here has ended. Tap “Ended” to see it.</p>
    </template>

    <div v-else-if="lanes" class="empty">
      <ChartGantt :size="40" class="muted" />
      <p>Nothing to show yet</p>
      <p class="muted small">Each subscription gets a lane: when it ran, paused and changed price.</p>
    </div>
  </main>
</template>

<style scoped>
.controls {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 4px 0 16px;
}
.chips {
  display: flex;
  gap: 8px;
  overflow-x: auto;
  scrollbar-width: none;
}
.quiet {
  text-align: center;
  margin-top: 24px;
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
