<script setup lang="ts">
// Home's filter: subscriptions or bills (when there are both), and any
// categories, each with what its running payments cost a year. Applies as
// you tap; "Show all" clears it.
import { Check, X } from 'lucide-vue-next'
import type { SubscriptionView } from '~/composables/api'
import { NO_CATEGORY as NONE, type HomeFilter } from '~/utils/homeFilter'

const props = defineProps<{ subs: SubscriptionView[]; names: Map<string, string>; main: string }>()
const filter = defineModel<HomeFilter>({ required: true })
const emit = defineEmits<{ close: [] }>()
useBackClose(() => emit('close'))

const money = (n: number) => formatMoney(n, props.main, { compact: true })
const yearly = (list: SubscriptionView[]) => list.reduce((sum, s) => sum + (s.yearlyInMain ?? 0), 0)
const bothKinds = computed(() => props.subs.some((s) => s.kind === 'bill') && props.subs.some((s) => s.kind === 'subscription'))
const kinds = computed(() =>
  (['all', 'subscription', 'bill'] as const).map((k) => ({
    key: k,
    label: { all: 'All', subscription: 'Subscriptions', bill: 'Bills' }[k],
    total: yearly(k === 'all' ? props.subs : props.subs.filter((s) => s.kind === k)),
  })),
)
// Categories in use, largest first; "No category" last.
const cats = computed(() => {
  const by = new Map<string, SubscriptionView[]>()
  for (const s of props.subs) {
    const k = s.categoryId && props.names.has(s.categoryId) ? s.categoryId : NONE
    by.set(k, [...(by.get(k) ?? []), s])
  }
  return [...by.entries()]
    .map(([key, list]) => ({ key, name: key === NONE ? 'No category' : props.names.get(key)!, total: yearly(list) }))
    .sort((a, b) => Number(a.key === NONE) - Number(b.key === NONE) || b.total - a.total)
})

function toggle(key: string) {
  const on = filter.value.categories.includes(key)
  filter.value = { ...filter.value, categories: on ? filter.value.categories.filter((c) => c !== key) : [...filter.value.categories, key] }
}
const active = computed(() => filter.value.kind !== 'all' || filter.value.categories.length > 0)
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <div class="sheet popover" role="dialog" aria-label="Filter">
      <header class="head">
        <b>Show</b>
        <button type="button" class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>

      <div v-if="bothKinds" class="chips" role="radiogroup" aria-label="Kind">
        <button
          v-for="k in kinds"
          :key="k.key"
          type="button"
          role="radio"
          class="chip"
          :aria-checked="filter.kind === k.key"
          :class="{ chosen: filter.kind === k.key }"
          @click="filter = { ...filter, kind: k.key }"
        >
          {{ k.label }} <span class="muted">{{ money(k.total) }}</span>
        </button>
      </div>

      <template v-if="cats.length > 1">
        <span class="muted small">Categories · a year</span>
        <ul class="pane rows list">
          <li v-for="c in cats" :key="c.key">
            <button type="button" class="row" role="checkbox" :aria-checked="filter.categories.includes(c.key)" @click="toggle(c.key)">
              <span class="checkbox" :class="{ checked: filter.categories.includes(c.key) }">
                <Check v-if="filter.categories.includes(c.key)" :size="14" :stroke-width="3" />
              </span>
              <span class="flex">{{ c.name }}</span>
              <span class="muted">{{ money(c.total) }}</span>
            </button>
          </li>
        </ul>
      </template>
      <p v-else class="muted small">Give your payments categories to filter by them.</p>

      <button v-if="active" type="button" class="btn" @click="filter = { kind: 'all', categories: [] }">Show all</button>
    </div>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: oklch(0 0 0 / 22%);
  display: flex;
  align-items: flex-end;
  animation: fade-in 0.15s;
}
.sheet {
  width: calc(100% - 16px);
  max-width: 520px;
  max-height: 85vh;
  overflow-y: auto;
  margin: 0 auto 8px;
  border-radius: 22px;
  padding: 8px 14px calc(14px + env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: 12px;
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-left: 4px;
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.list {
  margin: 0;
  list-style: none;
}
.row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-height: 48px;
  text-align: left;
}
.flex {
  flex: 1;
}
.small {
  margin: 0;
}
</style>
