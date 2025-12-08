import { mergeAttributes, Node } from '@tiptap/core';
import { VueNodeViewRenderer } from '@tiptap/vue-3';
import ImageNode from './ImageNode.vue';

export default Node.create({
  name: 'image',

  group: 'block', // Forces images to be blocks, solving the "same line" issue

  draggable: true,

  addAttributes() {
    return {
      src: {
        default: null,
      },
      alt: {
        default: null,
      },
      title: {
        default: null,
      },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'img[src]',
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return ['img', mergeAttributes(HTMLAttributes)];
  },

  addNodeView() {
    return VueNodeViewRenderer(ImageNode);
  },

  addStorage() {
    return {
      markdown: {
        serialize(state: any, node: any) {
          const src = node.attrs.src;
          // Decode the URI to ensure it's not double-encoded or unnecessarily encoded
          // e.g. "image%2054.png" -> "image 54.png"
          const decodedSrc = decodeURI(src);
          // Check if src contains spaces and is not already encoded, if so, we might want to keep it as is
          // or handle it specific to the user's preference.
          // The user specifically asked for "image 56.png" to remain "image 56.png"
          // Standard markdown often requires %20, but some parsers support spaces in <> or just spaces.
          // Tiptap's default serializer likely encodes it.
          // By manually writing it here with decodedSrc, we are respecting the user's wish.
          
          state.write(`![${node.attrs.alt || ''}](${decodedSrc})`);
          state.ensureNewLine();
          state.write('\n');
        }
      }
    }
  },

  addCommands() {
    return {
      setImage:
        (options: { src: string; alt?: string; title?: string }) =>
        ({ commands }) => {
          return commands.insertContent({
            type: this.name,
            attrs: options,
          });
        },
    };
  },
});
