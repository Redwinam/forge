<script setup lang="ts">
import { NodeViewWrapper, nodeViewProps } from '@tiptap/vue-3';
import { computed } from 'vue';

const props = defineProps(nodeViewProps);

const selected = computed(() => props.selected);
</script>

<template>
  <node-view-wrapper class="image-node-view" :class="{ 'is-selected': selected }">
    <div class="image-wrapper">
      <img
        :src="node.attrs.src"
        :alt="node.attrs.alt"
        :title="node.attrs.title"
      />
    </div>
  </node-view-wrapper>
</template>

<style lang="scss">
.image-node-view {
  display: block;
  margin: 0.5rem 0; // Reduced from 1.5rem to 0.5rem
  line-height: 0; // Prevent extra space for inline-block
  position: relative;
  
  // Selection state (blue ring)
  &.is-selected {
    .image-wrapper img {
      box-shadow: 0 0 0 3px #3b82f6; // Blue-500 ring
    }
  }

  .image-wrapper {
    display: inline-block;
    position: relative;
    max-width: 100%;

    img {
      display: block;
      max-width: 100%;
      height: auto;
      border-radius: 0.5rem;
      transition: box-shadow 0.15s ease-in-out;
    }
  }
}
</style>
