<script>
export default {
  name: 'OptionsApi',
  computed: {
    title() {
      return this.$t('page.title')
    },
    description() {
      return this.$t('page.description')
    },
    hasOptional() {
      return this.$te('optional')
    }
  }
}
</script>

<template>
  <div>
    <h2>{{ $t('page.title') }}</h2>
    <p>{{ $t('message.welcome', { name: 'World' }) }}</p>
  </div>
</template>
