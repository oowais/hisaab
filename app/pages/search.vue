<script setup lang="ts">
// Search payments: name, notes, link, plans, category and payment method;
// every word must appear. Running ones first, then ended, then archived.
// Tap a result to open it.
import { Search, X } from 'lucide-vue-next'
import type { SubscriptionView } from '~/composables/api'

const today = todayIso()
const query = ref('')
const hits = ref<SubscriptionView[]>([])
const searched = ref('')
const error = ref('')
const input = ref<HTMLInputElement>()

let seq = 0
async function run() {
  const q = query.value.trim()
  const mine = ++seq
  if (!q) {
    hits.value = []
    searched.value = ''
    return
  }
  try {
    const found = await api.search(q, today)
    // Typing on: only the latest answer counts.
    if (mine !== seq) return
    hits.value = found
    searched.value = q
    error.value = ''
  } catch (e) {
    error.value = friendlyError(e, 'search')
  }
}
let timer: ReturnType<typeof setTimeout> | undefined
watch(query, () => {
  clearTimeout(timer)
  timer = setTimeout(run, 120)
})
// A payment changed (here or by sync): results follow.
watch(dataVersion, run)

const running = computed(() => hits.value.filter((h) => !h.archivedAt && h.status !== 'ended'))
const over = computed(() => hits.value.filter((h) => h.archivedAt || h.status === 'ended'))

/** "€9.99 in 3 days", "Paused", "Ended 3 Mar 2025", "Archived". */
function meta(s: SubscriptionView) {
  if (s.archivedAt) return 'Archived'
  if (s.status === 'ended') return s.current?.endDate ? `Ended ${shortDay(s.current.endDate, today)}` : 'Ended'
  if (s.status === 'paused') return 'Paused'
  const c = s.nextCharge
  if (!c) return s.status === 'trial' ? 'Free trial' : ''
  return `${formatMoney(c.amountMinor, c.currency)} ${relativeDay(c.dueDate, today)}`
}

function clear() {
  query.value = ''
  input.value?.focus()
}

onMounted(() => input.value?.focus())
</script>

<template>
  <Screen title="Search">
    <label class="box">
      <Search :size="20" class="muted" />
      <input
        ref="input"
        v-model="query"
        class="field"
        type="search"
        enterkeyhint="search"
        placeholder="Name, notes, plan, category"
        autocomplete="off"
        @keydown.esc="query ? clear() : $router.back()"
      />
      <button v-if="query" class="icon-btn clear" aria-label="Clear" @click="clear"><X :size="18" /></button>
    </label>

    <p v-if="error" class="error">{{ error }}</p>

    <ul v-if="running.length" class="pane rows list">
      <li v-for="s in running" :key="s.id">
        <NuxtLink :to="`/sub/${s.id}`" class="sub-row">
          <SubIcon :icon="s.icon" :color="s.color" :name="s.name" :size="38" />
          <span class="stack">
            <span>{{ s.name }}</span>
            <span class="muted small">{{ meta(s) }}</span>
          </span>
        </NuxtLink>
      </li>
    </ul>

    <template v-if="over.length">
      <h2 class="section-title">Ended and archived</h2>
      <ul class="pane rows list">
        <li v-for="s in over" :key="s.id">
          <NuxtLink :to="`/sub/${s.id}`" class="sub-row">
            <SubIcon :icon="s.icon" :color="s.color" :name="s.name" :size="38" />
            <span class="stack">
              <span>{{ s.name }}</span>
              <span class="muted small">{{ meta(s) }}</span>
            </span>
          </NuxtLink>
        </li>
      </ul>
    </template>

    <p v-if="searched && !hits.length" class="muted empty">Nothing matches "{{ searched }}".</p>
  </Screen>
</template>

<style scoped>
.box {
  position: relative;
  display: flex;
  align-items: center;
  margin: 4px 0 12px;
}
.box > .muted {
  position: absolute;
  left: 14px;
  pointer-events: none;
}
.box .field {
  padding-left: 44px;
  padding-right: 44px;
  font-size: 1.05rem;
}
/* Our own clear button, not the browser's. */
.box .field::-webkit-search-cancel-button {
  display: none;
}
.clear {
  position: absolute;
  right: 2px;
}
.list {
  margin: 0;
}
.list li {
  list-style: none;
}
.sub-row {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 56px;
  color: inherit;
  text-decoration: none;
}
.stack {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.empty {
  margin-top: 20px;
  text-align: center;
}
</style>
