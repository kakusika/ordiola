import { Node, mergeAttributes } from "@tiptap/core";

export interface CalloutOptions {
  HTMLAttributes: Record<string, any>;
}

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    callout: {
      setCallout: (attributes?: { type: string }) => ReturnType;
      toggleCallout: (attributes?: { type: string }) => ReturnType;
    };
  }
}

export const Callout = Node.create<CalloutOptions>({
  name: "callout",

  group: "block",

  content: "block+",

  defining: true,

  addOptions() {
    return {
      HTMLAttributes: {},
    };
  },

  addAttributes() {
    return {
      type: {
        default: "note",
        parseHTML: (element) => element.getAttribute("data-callout") || "note",
        renderHTML: (attributes) => ({
          "data-callout": attributes.type,
        }),
      },
    };
  },

  parseHTML() {
    return [
      {
        tag: "div[data-callout]",
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    const type = HTMLAttributes["data-callout"] || "note";
    return [
      "div",
      mergeAttributes(this.options.HTMLAttributes, HTMLAttributes, {
        class: `tomet-callout-card tomet-callout-${type}`,
      }),
      0,
    ];
  },

  addCommands() {
    return {
      setCallout:
        (attributes) =>
        ({ commands }) => {
          return commands.setNode(this.name, attributes);
        },
      toggleCallout:
        (attributes) =>
        ({ commands }) => {
          return commands.toggleNode(this.name, "paragraph", attributes);
        },
    };
  },
});
