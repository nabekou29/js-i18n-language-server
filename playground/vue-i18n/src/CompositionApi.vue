<script setup lang="ts">
import { useI18n } from 'vue-i18n'

// Destructured form
const { t, te, tm } = useI18n()

const greeting = t('message.hello')
const exists = te('optional')
const raw = tm('message')
</script>

<template>
  <div>
    <h2>{{ $t('page.title') }}</h2>
    <p>{{ $t('page.description') }}</p>

    <!-- Attribute binding -->
    <input :placeholder="$t('form.placeholder')" />

    <!-- Conditional rendering -->
    <span v-if="$te('optional')">{{ $t('optional') }}</span>
    <span v-show="$te('visible')">{{ $t('visible') }}</span>

    <!-- Event handler -->
    <button @click="alert($t('message.hello'))">{{ $t('form.submit') }}</button>
  </div>
</template>
