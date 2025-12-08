<script setup lang="ts">
import { 
  Bold, Italic, Strikethrough, Code, List, ListOrdered, Quote, Undo, Redo, 
  Image as ImageIcon, Eye, EyeOff, Heading1, Heading2, Heading3, 
  Underline as UnderlineIcon, AlignLeft, AlignCenter, AlignRight, AlignJustify, Link as LinkIcon,
  CheckSquare, Table as TableIcon
} from 'lucide-vue-next';
import { Editor } from '@tiptap/vue-3';

defineProps<{
  editor: Editor | undefined;
  showMetadata: boolean;
}>();

const emit = defineEmits<{
  (e: 'toggle-metadata'): void;
  (e: 'upload-image'): void;
  (e: 'set-link'): void;
  (e: 'insert-table'): void;
}>();
</script>

<template>
  <div v-if="editor" class="flex items-center gap-1 p-2 border-b border-gray-200 bg-white sticky top-0 z-10 overflow-x-auto shadow-sm">
    <!-- History -->
    <div class="flex items-center gap-1 pr-2 border-r border-gray-200">
      <button @click="editor.chain().focus().undo().run()" :disabled="!editor.can().undo()" class="p-1.5 rounded hover:bg-gray-100 disabled:opacity-50 text-gray-600" title="Undo">
        <Undo :size="18" />
      </button>
      <button @click="editor.chain().focus().redo().run()" :disabled="!editor.can().redo()" class="p-1.5 rounded hover:bg-gray-100 disabled:opacity-50 text-gray-600" title="Redo">
        <Redo :size="18" />
      </button>
    </div>

    <!-- Headings -->
    <div class="flex items-center gap-1 px-2 border-r border-gray-200">
      <button @click="editor.chain().focus().toggleHeading({ level: 1 }).run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('heading', { level: 1 }) }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Heading 1">
        <Heading1 :size="18" />
      </button>
      <button @click="editor.chain().focus().toggleHeading({ level: 2 }).run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('heading', { level: 2 }) }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Heading 2">
        <Heading2 :size="18" />
      </button>
      <button @click="editor.chain().focus().toggleHeading({ level: 3 }).run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('heading', { level: 3 }) }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Heading 3">
        <Heading3 :size="18" />
      </button>
    </div>

    <!-- Formatting -->
    <div class="flex items-center gap-1 px-2 border-r border-gray-200">
      <button @click="editor.chain().focus().toggleBold().run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('bold') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Bold">
        <Bold :size="18" />
      </button>
      <button @click="editor.chain().focus().toggleItalic().run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('italic') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Italic">
        <Italic :size="18" />
      </button>
      <button @click="editor.chain().focus().toggleUnderline().run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('underline') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Underline">
        <UnderlineIcon :size="18" />
      </button>
      <button @click="editor.chain().focus().toggleStrike().run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('strike') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Strikethrough">
        <Strikethrough :size="18" />
      </button>
      <button @click="editor.chain().focus().toggleCode().run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('code') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Code">
        <Code :size="18" />
      </button>
    </div>

    <!-- Alignment -->
    <div class="flex items-center gap-1 px-2 border-r border-gray-200">
      <button @click="editor.chain().focus().setTextAlign('left').run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive({ textAlign: 'left' }) }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Align Left">
        <AlignLeft :size="18" />
      </button>
      <button @click="editor.chain().focus().setTextAlign('center').run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive({ textAlign: 'center' }) }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Align Center">
        <AlignCenter :size="18" />
      </button>
      <button @click="editor.chain().focus().setTextAlign('right').run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive({ textAlign: 'right' }) }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Align Right">
        <AlignRight :size="18" />
      </button>
    </div>

    <!-- Lists -->
    <div class="flex items-center gap-1 px-2 border-r border-gray-200">
      <button @click="editor.chain().focus().toggleBulletList().run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('bulletList') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Bullet List">
        <List :size="18" />
      </button>
      <button @click="editor.chain().focus().toggleOrderedList().run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('orderedList') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Ordered List">
        <ListOrdered :size="18" />
      </button>
      <button @click="editor.chain().focus().toggleTaskList().run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('taskList') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Task List">
        <CheckSquare :size="18" />
      </button>
      <button @click="editor.chain().focus().toggleBlockquote().run()" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('blockquote') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Quote">
        <Quote :size="18" />
      </button>
    </div>

    <!-- Insert -->
    <div class="flex items-center gap-1 px-2">
      <button @click="$emit('insert-table')" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('table') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Insert Table">
        <TableIcon :size="18" />
      </button>
      <button @click="$emit('set-link')" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('link') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Link">
        <LinkIcon :size="18" />
      </button>
      <button @click="$emit('upload-image')" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Upload Image">
        <ImageIcon :size="18" />
      </button>
    </div>
    
    <div class="flex-1"></div>
    
    <button @click="$emit('toggle-metadata')" class="p-1.5 rounded hover:bg-gray-100 mr-2 text-gray-600" :title="showMetadata ? 'Hide Metadata' : 'Show Metadata'">
      <Eye v-if="showMetadata" :size="18" />
      <EyeOff v-else :size="18" />
    </button>
  </div>
</template>