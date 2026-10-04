<template>
  <button type="button" class="icon-button" :aria-label="String($attrs.title ?? '操作')">
    <slot />
  </button>
</template>
