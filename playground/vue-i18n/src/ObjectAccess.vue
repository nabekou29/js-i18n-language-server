<script setup>
import { useI18n } from 'vue-i18n'

// Object access pattern
const i18n = useI18n()
const title = i18n.t('page.title')
</script>

<template>
  <div>
    <h2>{{ $t('page.title') }}</h2>
  </div>
</template>
