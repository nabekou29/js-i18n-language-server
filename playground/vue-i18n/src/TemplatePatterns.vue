<script setup>
import { useI18n } from 'vue-i18n'
const { t } = useI18n()
</script>

<template>
  <div>
    <!-- i18n-t component (v9+) -->
    <i18n-t keypath="terms" tag="p">
      <template #link>
        <a href="/tos">{{ $t('tos') }}</a>
      </template>
    </i18n-t>

    <!-- I18nT PascalCase -->
    <I18nT keypath="terms" tag="p">
      <template #link>
        <a href="/tos">{{ $t('tos') }}</a>
      </template>
    </I18nT>

    <!-- i18n component (v8) -->
    <i18n path="terms" tag="p">
      <a slot="link" href="/tos">{{ $t('tos') }}</a>
    </i18n>

    <!-- v-t directive: string syntax -->
    <p v-t="'message.hello'"></p>

    <!-- v-t directive: object syntax -->
    <p v-t="{ path: 'message.goodbye', args: { name: 'World' } }"></p>
  </div>
</template>
