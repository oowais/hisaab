<script setup lang="ts">
// A payment's own screen (phones and narrow windows). On a wide window it
// opens beside Upcoming instead (`/?sub=<id>`, keeping ?period, ?action,
// ?snooze).
const route = useRoute()
const router = useRouter()
const id = computed(() => String(route.params.id))
const wide = useWide()
watch(
  wide,
  (w) => {
    if (w) router.replace({ path: '/', query: { ...route.query, sub: id.value } })
  },
  { immediate: true },
)
</script>

<template>
  <SubDetail v-if="!wide" :id="id" />
</template>
