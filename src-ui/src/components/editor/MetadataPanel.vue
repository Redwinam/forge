<script setup lang="ts">
defineProps<{
  metadata: Record<string, any>;
  show: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:metadata', value: Record<string, any>): void;
}>();
</script>

<template>
  <div v-if="show && Object.keys(metadata).length > 0" class="bg-gray-50 border-b border-gray-200 transition-all duration-300 ease-in-out">
    <div class="px-8 py-4">
      <div class="flex items-center gap-2 mb-3 text-gray-400">
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><polyline points="10 9 9 9 8 9"/></svg>
        <span class="text-[10px] font-bold uppercase tracking-widest">Frontmatter</span>
      </div>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-6 gap-y-4">
        <div v-for="(value, key) in metadata" :key="key" class="group">
          <label class="block text-[10px] font-bold text-gray-500 uppercase tracking-wider mb-1">{{ key }}</label>
          <div class="relative">
            <input 
              v-if="typeof value === 'string' || typeof value === 'number'"
              v-model="metadata[key]" 
              class="w-full bg-white text-gray-800 text-xs font-medium px-3 py-2 rounded border border-gray-200 transition-all duration-200 hover:border-blue-400 focus:border-blue-500 focus:ring-2 focus:ring-blue-100 outline-none"
              :placeholder="`Enter ${key}...`"
            />
            <div v-else class="text-xs text-gray-500 italic bg-gray-100 px-3 py-2 rounded border border-gray-200">
              <span class="inline-flex items-center gap-1.5">
                <span class="w-1.5 h-1.5 rounded-full bg-yellow-400"></span>
                Complex value
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>