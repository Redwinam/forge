import type { AnyExtension } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';
import { Markdown } from '@tiptap/markdown';
import Placeholder from '@tiptap/extension-placeholder';
import TextAlign from '@tiptap/extension-text-align';
import TaskList from '@tiptap/extension-task-list';
import TaskItem from '@tiptap/extension-task-item';
import { Table } from '@tiptap/extension-table';
import { TableCell } from '@tiptap/extension-table-cell';
import { TableHeader } from '@tiptap/extension-table-header';
import { TableRow } from '@tiptap/extension-table-row';
import CustomImage from './image-extension';
import { useLenientEscaping } from './markdown-escape';

// Shared by the editor and the markdown round-trip tests, so the tests serialise exactly
// what the app writes to disk.
export function editorExtensions(): AnyExtension[] {
  return [
    StarterKit.configure({
      heading: {
        levels: [1, 2, 3, 4, 5, 6],
      },
      link: {
        openOnClick: false,
      },
    }),
    Markdown.extend({
      onBeforeCreate(event) {
        this.parent?.(event);
        useLenientEscaping(this.editor);
      },
    }).configure({
      markedOptions: {
        breaks: true, // Convert newlines to hard breaks
      },
    }),
    CustomImage,
    Placeholder.configure({
      placeholder: "Type '/' for commands",
    }),
    TextAlign.configure({
      types: ['heading', 'paragraph'],
    }),
    TaskList.configure({
      HTMLAttributes: {
        class: 'not-prose pl-2',
      },
    }),
    TaskItem.configure({
      nested: true,
      HTMLAttributes: {
        class: 'task-list-item', // Use a dedicated class for styling
      },
    }),
    Table.configure({
      resizable: true,
      HTMLAttributes: {
        class: 'min-w-full border-collapse border border-gray-300 my-4',
      },
    }),
    TableRow.configure({
      HTMLAttributes: {
        class: 'border-b border-gray-300',
      },
    }),
    TableHeader.configure({
      HTMLAttributes: {
        class: 'border border-gray-300 bg-gray-100 px-4 py-2 font-bold text-left',
      },
    }),
    TableCell.configure({
      HTMLAttributes: {
        class: 'border border-gray-300 px-4 py-2',
      },
    }),
  ];
}
