<script setup lang="ts">
// Edit what a subscription is (not its price: that's "Change price").
import { X } from 'lucide-vue-next'
import type { Detail } from '~/composables/api'

const props = defineProps<{ sub: Detail }>()
const emit = defineEmits<{ close: []; done: [] }>()
useBackClose(() => emit('close'))

const s = props.sub
const name = ref(s.name)
const kind = ref(s.kind)
const url = ref(s.url ?? '')
const notes = ref(s.notes)
const icon = ref(s.icon)
const color = ref(s.color)
const pickingIcon = ref(false)
const myShare = ref<number | null>(s.myShare)
const remindDays = ref(s.remindDaysBefore == null ? '' : String(s.remindDaysBefore))
const error = ref('')

async function save() {
  const days = remindDays.value.trim() === '' ? null : Number(remindDays.value)
  try {
    await api.updateSubscription(s.id, {
      kind: kind.value,
      name: name.value,
      templateKey: s.templateKey,
      icon: icon.value,
      color: color.value,
      categoryId: s.categoryId,
      paymentMethodId: s.paymentMethodId,
      url: url.value.trim() || null,
      notes: notes.value.trim(),
      remindDaysBefore: days != null && Number.isInteger(days) && days >= 0 ? days : null,
      myShare: myShare.value,
    })
    changed()
    emit('done')
  } catch (e) {
    error.value = friendlyError(e, 'save')
  }
}
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <form class="sheet popover" role="dialog" aria-label="Edit" @submit.prevent="save">
      <header class="head">
        <b>Edit</b>
        <button type="button" class="icon-btn" aria-label="Close" @click="emit('close')"><X :size="22" /></button>
      </header>
      <div class="name-row">
        <button type="button" class="tile-btn" aria-label="Change icon and colour" @click="pickingIcon = true">
          <SubIcon :icon="icon" :color="color" :name="name" :size="44" />
        </button>
        <input v-model="name" class="field" aria-label="Name" />
      </div>
      <div class="chips">
        <button type="button" class="chip" :class="{ chosen: kind === 'subscription' }" @click="kind = 'subscription'">Subscription</button>
        <button type="button" class="chip" :class="{ chosen: kind === 'bill' }" @click="kind = 'bill'">Bill</button>
      </div>
      <span class="muted small">Shared with others? Your part</span>
      <ShareChips v-model="myShare" />
      <input v-model="url" class="field" type="url" inputmode="url" placeholder="Website or cancel link" aria-label="Link" />
      <input
        v-model="remindDays"
        class="field"
        type="number"
        inputmode="numeric"
        min="0"
        max="60"
        placeholder="Remind days before (default)"
        aria-label="Remind days before"
      />
      <textarea v-model="notes" class="field" rows="3" placeholder="Notes" aria-label="Notes" />
      <p v-if="error" class="error">{{ error }}</p>
      <button type="submit" class="btn primary">Save</button>
    </form>
    <IconSheet
      v-if="pickingIcon"
      :icon="icon"
      :color="color"
      :name="name"
      @pick="(i, c) => ((icon = i), (color = c))"
      @close="pickingIcon = false"
    />
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
  gap: 8px;
}
textarea {
  resize: vertical;
}
.error {
  margin: 0;
}
.name-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.name-row .field {
  flex: 1;
}
.tile-btn {
  padding: 0;
  display: inline-flex;
}
</style>
