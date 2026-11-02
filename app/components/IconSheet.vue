<script setup lang="ts">
// Pick a subscription's icon and colour: a preview, colour swatches, then
// the first letter, generic icons and brand glyphs, filterable. Picking a
// brand also takes its brand colour (a swatch after that overrides it).
import { Check, X } from 'lucide-vue-next'

const props = defineProps<{ icon: string | null; color: string | null; name: string }>()
const emit = defineEmits<{ pick: [icon: string | null, color: string | null]; close: [] }>()
useBackClose(() => emit('close'))
const drag = useSheetDrag(() => emit('close'))

const icon = ref(props.icon)
const color = ref(props.color)
const query = ref('')
const q = computed(() => query.value.trim().toLowerCase())

const generic = computed(() =>
  Object.keys(GENERIC_ICONS)
    .filter((k) => !q.value || k.toLowerCase().includes(q.value))
    .sort(),
)
const brands = computed(() => BRANDS.filter((b) => !q.value || b.title.toLowerCase().includes(q.value)))

function pickBrand(slug: string, hex: string) {
  icon.value = `si:${slug}`
  color.value = hex
}
function done() {
  emit('pick', icon.value, color.value)
  emit('close')
}
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <div
      class="sheet popover"
      :class="{ dragging: drag.dragging.value }"
      :style="drag.style.value"
      v-on="drag.handlers"
      role="dialog"
      aria-label="Icon and colour"
    >
      <header class="head">
        <SubIcon :icon="icon" :color="color" :name="name" :size="44" />
        <!-- :value + @input, not v-model: v-model waits for the Android keyboard to commit the word. -->
        <input
          :value="query"
          class="field"
          placeholder="Search icons"
          aria-label="Search icons"
          @input="query = ($event.target as HTMLInputElement).value"
        />
        <button class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>

      <div class="swatches" role="radiogroup" aria-label="Colour">
        <button
          v-for="c in PALETTE"
          :key="c"
          class="swatch"
          role="radio"
          :aria-checked="color?.toLowerCase() === c"
          :aria-label="c"
          :style="{ background: c, color: glyphOn(c) }"
          @click="color = c"
        >
          <Check v-if="color?.toLowerCase() === c" :size="16" :stroke-width="3" />
        </button>
      </div>

      <div class="scroll">
        <div class="grid">
          <button v-if="!q" class="cell" :class="{ chosen: !icon }" aria-label="First letter" @click="icon = null">
            <SubIcon :icon="null" :color="color" :name="name" :size="36" />
          </button>
          <button
            v-for="k in generic"
            :key="k"
            class="cell"
            :class="{ chosen: icon === `lu:${k}` }"
            :aria-label="k"
            @click="icon = `lu:${k}`"
          >
            <SubIcon :icon="`lu:${k}`" :color="color" :name="name" :size="36" />
          </button>
        </div>
        <h3 v-if="brands.length" class="section-title">Brands</h3>
        <div class="grid">
          <button
            v-for="b in brands"
            :key="b.slug"
            class="cell"
            :class="{ chosen: icon === `si:${b.slug}` }"
            :aria-label="b.title"
            :title="b.title"
            @click="pickBrand(b.slug, b.hex)"
          >
            <SubIcon :icon="`si:${b.slug}`" :color="b.hex" :name="b.title" :size="36" />
          </button>
        </div>
      </div>

      <button class="btn primary" @click="done"><Check :size="18" /> Done</button>
    </div>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 60;
  background: oklch(0 0 0 / 22%);
  display: flex;
  align-items: flex-end;
  animation: fade-in 0.15s;
}
.sheet {
  width: calc(100% - 16px);
  max-width: 520px;
  height: 80vh;
  margin: 0 auto 8px;
  border-radius: 22px;
  padding: 12px 12px calc(12px + env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: 12px;
  animation: float-in 0.24s cubic-bezier(0.22, 1, 0.36, 1);
}
.head {
  display: flex;
  align-items: center;
  gap: 10px;
}
.head .field {
  flex: 1;
  min-width: 0;
}
.swatches {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.swatch {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  box-shadow: inset 0 0 0 1px oklch(0 0 0 / 12%);
}
.scroll {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(52px, 1fr));
  gap: 6px;
}
.cell {
  display: grid;
  place-items: center;
  height: 52px;
  border-radius: 14px;
}
.cell.chosen {
  box-shadow: 0 0 0 2px var(--accent);
}
.section-title {
  margin: 16px 0 8px;
}
</style>
