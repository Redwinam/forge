<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue';
import { useEditor, EditorContent } from '@tiptap/vue-3';
import StarterKit from '@tiptap/starter-kit';
import { Markdown } from 'tiptap-markdown';
import Image from '@tiptap/extension-image';
import Link from '@tiptap/extension-link';
import Placeholder from '@tiptap/extension-placeholder';
import Underline from '@tiptap/extension-underline';
import TextAlign from '@tiptap/extension-text-align';
import TaskList from '@tiptap/extension-task-list';
import TaskItem from '@tiptap/extension-task-item';
import { Table } from '@tiptap/extension-table';
import { TableCell } from '@tiptap/extension-table-cell';
import { TableHeader } from '@tiptap/extension-table-header';
import { TableRow } from '@tiptap/extension-table-row';
import { invoke } from '@tauri-apps/api/core';
import COS from 'cos-js-sdk-v5';
import SparkMD5 from 'spark-md5';
import { 
  Bold, Italic, Strikethrough, Code, List, ListOrdered, Quote, Undo, Redo, 
  Image as ImageIcon, Eye, EyeOff, Heading1, Heading2, Heading3, 
  Underline as UnderlineIcon, AlignLeft, AlignCenter, AlignRight, AlignJustify, Link as LinkIcon,
  CheckSquare, Table as TableIcon
} from 'lucide-vue-next';
import yaml from 'js-yaml';

interface CosConfig {
  secret_id: string;
  secret_key: string;
  bucket: string;
  region: string;
  prefix: string;
}

const props = defineProps<{
  content: string;
  filePath: string | null;
}>();

const emit = defineEmits<{
  (e: 'save', content: string): void;
}>();

const cosConfig = ref<CosConfig | null>(null);
const isUploading = ref(false);
const metadata = ref<Record<string, any>>({});
const showMetadata = ref(true);

// Parse frontmatter
const parseContent = (fullContent: string) => {
  const frontmatterRegex = /^---\r?\n([\s\S]*?)\r?\n---\r?\n([\s\S]*)$/;
  const match = fullContent.match(frontmatterRegex);

  if (match) {
    try {
      const yamlContent = match[1];
      const bodyContent = match[2];
      const parsedData = yaml.load(yamlContent || '') as Record<string, any>;
      
      metadata.value = parsedData || {};
      return bodyContent || '';
    } catch (e) {
      console.error("Failed to parse YAML", e);
      metadata.value = {};
      return fullContent;
    }
  }

  metadata.value = {};
  return fullContent;
};

const stringifyContent = (body: string) => {
  if (Object.keys(metadata.value).length === 0) return body;
  try {
    const yamlString = yaml.dump(metadata.value);
    return `---\n${yamlString}---\n${body}`;
  } catch (e) {
    console.error("Failed to stringify YAML", e);
    return body;
  }
};

// Removed props.envPath check
onMounted(() => {
  // No longer loading config from envPath here
});

const uploadImage = async (file: File): Promise<string> => {
  // Using new Rust command instead of JS SDK
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = async (e) => {
      if (e.target?.result) {
        try {
          // Convert ArrayBuffer to number array for Rust
          const arrayBuffer = e.target.result as ArrayBuffer;
          const bytes = Array.from(new Uint8Array(arrayBuffer));
          
          // Get extension
          const ext = file.name.split('.').pop() || 'png';
          
          // Call Rust command
          const url = await invoke<string>('upload_image', {
            fileData: bytes,
            extension: ext
          });
          resolve(url);
        } catch (err) {
          reject(err);
        }
      } else {
        reject("Failed to read file");
      }
    };
    reader.onerror = (e) => reject(e);
    reader.readAsArrayBuffer(file);
  });
};

const editor = useEditor({
  content: parseContent(props.content),
  extensions: [
    StarterKit.configure({
      heading: {
        levels: [1, 2, 3, 4, 5, 6],
      },
    }),
    Markdown,
    Image,
    Link.configure({
      openOnClick: false,
    }),
    Placeholder.configure({
      placeholder: "Type '/' for commands",
    }),
    Underline,
    TextAlign.configure({
      types: ['heading', 'paragraph'],
    }),
    TaskList,
    TaskItem.configure({
      nested: true,
    }),
    Table.configure({
      resizable: true,
    }),
    TableRow,
    TableHeader,
    TableCell,
  ],
  editorProps: {
    attributes: {
      class: 'prose prose-slate max-w-none focus:outline-none min-h-[calc(100vh-200px)] px-8 py-6'
    },
    handlePaste: (_view, event) => {
      const items = Array.from(event.clipboardData?.items || []);
      const item = items.find(item => item.type.indexOf('image') === 0);
      if (item) {
        event.preventDefault();
        const file = item.getAsFile();
        if (file) {
          isUploading.value = true;
          uploadImage(file)
            .then(url => {
              editor.value?.chain().focus().setImage({ src: url }).run();
            })
            .catch(err => alert("Upload failed: " + err))
            .finally(() => isUploading.value = false);
        }
        return true;
      }
      return false;
    },
    handleDrop: (_view, event, _slice, moved) => {
      if (!moved && event.dataTransfer && event.dataTransfer.files && event.dataTransfer.files.length > 0) {
        const files = Array.from(event.dataTransfer.files);
        const image = files.find(file => file.type.indexOf('image') === 0);
        if (image) {
          event.preventDefault();
          isUploading.value = true;
          uploadImage(image)
            .then(url => {
              editor.value?.chain().focus().setImage({ src: url }).run();
            })
            .catch(err => alert("Upload failed: " + err))
            .finally(() => isUploading.value = false);
          return true;
        }
      }
      return false;
    }
  }
});

// Update content when props change
watch(() => props.content, (newContent) => {
  const newBody = parseContent(newContent);
  if (editor.value && newBody !== (editor.value.storage as any).markdown.getMarkdown()) {
    editor.value.commands.setContent(newBody || '');
  }
});

const setLink = () => {
  if (!editor.value) return;
  const previousUrl = editor.value.getAttributes('link').href;
  const url = window.prompt('URL', previousUrl);

  // cancelled
  if (url === null) {
    return;
  }

  // empty
  if (url === '') {
    editor.value.chain().focus().extendMarkRange('link').unsetLink().run();
    return;
  }

  // update
  editor.value.chain().focus().extendMarkRange('link').setLink({ href: url }).run();
};

const insertTable = () => {
  editor.value?.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run();
};

const handleImageUploadClick = () => {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = 'image/*';
  input.onchange = async (e) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (file) {
      isUploading.value = true;
      try {
        const url = await uploadImage(file);
        editor.value?.chain().focus().setImage({ src: url }).run();
      } catch (err) {
        alert("Upload failed: " + err);
      } finally {
        isUploading.value = false;
      }
    }
  };
  input.click();
};

const handleKeyDown = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === 's') {
    e.preventDefault();
    if (editor.value) {
      const markdownBody = (editor.value.storage as any).markdown.getMarkdown();
      const fullContent = stringifyContent(markdownBody);
      emit('save', fullContent);
    }
  }
};

onMounted(() => {
  window.addEventListener('keydown', handleKeyDown);
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeyDown);
  editor.value?.destroy();
});
</script>

<template>
  <div class="flex flex-col h-full relative bg-white">
    <!-- Toolbar -->
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
        <button @click="insertTable" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('table') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Insert Table">
          <TableIcon :size="18" />
        </button>
        <button @click="setLink" :class="{ 'bg-blue-100 text-blue-600': editor.isActive('link') }" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Link">
          <LinkIcon :size="18" />
        </button>
        <button @click="handleImageUploadClick" class="p-1.5 rounded hover:bg-gray-100 text-gray-600" title="Upload Image">
          <ImageIcon :size="18" />
        </button>
      </div>
      
      <div class="flex-1"></div>
      
      <button @click="showMetadata = !showMetadata" class="p-1.5 rounded hover:bg-gray-100 mr-2 text-gray-600" :title="showMetadata ? 'Hide Metadata' : 'Show Metadata'">
        <Eye v-if="showMetadata" :size="18" />
        <EyeOff v-else :size="18" />
      </button>
    </div>

    <!-- Metadata Panel -->
    <div v-if="showMetadata && Object.keys(metadata).length > 0" class="bg-gray-50 border-b border-gray-200 transition-all duration-300 ease-in-out">
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

    <div v-if="isUploading" class="absolute top-14 right-4 z-20 bg-blue-100 text-blue-700 px-3 py-1 rounded-full text-sm shadow-md animate-pulse">
      Uploading image...
    </div>

    <editor-content :editor="editor" class="flex-1 overflow-y-auto" />
  </div>
</template>

<style>
/* Custom Scrollbar */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: #cbd5e1;
  border-radius: 4px;
}
::-webkit-scrollbar-thumb:hover {
  background: #94a3b8;
}

/* Editor Specifics */
.ProseMirror {
  outline: none;
  min-height: 100%;
}

.ProseMirror p.is-editor-empty:first-child::before {
  color: #94a3b8;
  content: attr(data-placeholder);
  float: left;
  height: 0;
  pointer-events: none;
}

/* Typography Overrides */
.ProseMirror h1,
.ProseMirror h2,
.ProseMirror h3,
.ProseMirror h4,
.ProseMirror h5,
.ProseMirror h6 {
  line-height: 1.1;
  margin-top: 2.5rem;
  margin-bottom: 0.5rem;
  text-wrap: pretty;
}

.ProseMirror h1 {
  font-size: 2.25em;
  font-weight: 800;
  color: #111827;
}
.ProseMirror h2 {
  font-size: 1.5em;
  font-weight: 700;
  color: #1f2937;
}
.ProseMirror h3 {
  font-size: 1.25em;
  font-weight: 600;
  color: #374151;
}

.ProseMirror p {
  margin-top: 0.75em;
  margin-bottom: 0.75em;
  line-height: 1.75;
}

/* Remove backticks from inline code in Tailwind Typography */
.prose :where(code):not(:where([class~="not-prose"] *))::before {
  content: "" !important;
}
.prose :where(code):not(:where([class~="not-prose"] *))::after {
  content: "" !important;
}

.ProseMirror code {
  background-color: #f1f5f9;
  color: #0f172a;
  border-radius: 0.25rem;
  padding: 0.25rem 0.375rem;
  font-size: 0.875em;
  font-weight: 500;
}

.ProseMirror pre {
  background: #1f2937;
  color: #e5e7eb;
  font-family: 'JetBrainsMono', monospace;
  padding: 0.75rem 1rem;
  border-radius: 0.5rem;
  margin: 1.5rem 0;
}
.ProseMirror pre code {
  color: inherit;
  padding: 0;
  background: none;
  font-size: 0.8rem;
  font-weight: normal;
}

/* Task Lists */
ul[data-type="taskList"] {
  list-style: none;
  padding: 0;
  margin: 0;
}

li[data-type="taskItem"] {
  display: flex;
  gap: 0.5rem;
  align-items: flex-start;
  margin-bottom: 0.5rem;
}

li[data-type="taskItem"] input[type="checkbox"] {
  margin-top: 0.3rem;
  cursor: pointer;
}

li[data-type="taskItem"] > div {
  flex: 1;
}

/* Blockquote */
.ProseMirror blockquote {
  border-left: 3px solid #e2e8f0;
  padding-left: 1rem;
  font-style: italic;
  color: #4b5563;
  margin-left: 0;
  margin-right: 0;
}

/* Images */
.ProseMirror img {
  max-width: 100%;
  height: auto;
  border-radius: 0.5rem;
  margin: 1.5rem 0;
  box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05);
}

/* Links */
.ProseMirror a {
  color: #2563eb;
  text-decoration: none;
  cursor: pointer;
}
.ProseMirror a:hover {
  text-decoration: underline;
}

.ProseMirror ul,
.ProseMirror ol {
  padding-left: 1.5rem;
}

/* Tables */
.ProseMirror table {
  border-collapse: collapse;
  table-layout: fixed;
  width: 100%;
  margin: 0;
  overflow: hidden;
}

.ProseMirror td,
.ProseMirror th {
  min-width: 1em;
  border: 1px solid #ced4da;
  padding: 0.5rem;
  vertical-align: top;
  box-sizing: border-box;
  position: relative;
}

.ProseMirror th {
  font-weight: 600;
  text-align: left;
  background-color: #f8f9fa;
}

.ProseMirror .selectedCell:after {
  z-index: 2;
  position: absolute;
  content: "";
  left: 0; right: 0; top: 0; bottom: 0;
  background: rgba(200, 200, 255, 0.4);
  pointer-events: none;
}

.ProseMirror .column-resize-handle {
  position: absolute;
  right: -2px;
  top: 0;
  bottom: -2px;
  width: 4px;
  background-color: #adf;
  pointer-events: none;
}
</style>
