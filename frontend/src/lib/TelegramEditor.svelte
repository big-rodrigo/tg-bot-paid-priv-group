<script lang="ts">
  import { Editor } from '@tiptap/core';
  import StarterKit from '@tiptap/starter-kit';
  import Underline from '@tiptap/extension-underline';
  import Link from '@tiptap/extension-link';
  import { fromTelegramHtml } from './telegramify';

  interface Props {
    /** Initial content as Telegram HTML (stored format). */
    content?: string;
    /** Called whenever the editor content changes. Receives raw Tiptap HTML. */
    onchange?: (html: string) => void;
    placeholder?: string;
  }

  let { content = '', onchange, placeholder = 'Enter text…' }: Props = $props();

  let editorInstance: Editor | undefined;
  let isEmpty = $state(true);

  function mountEditor(node: HTMLDivElement) {
    const editor = new Editor({
      element: node,
      extensions: [
        StarterKit.configure({ heading: false, horizontalRule: false }),
        Underline,
        Link.configure({ openOnClick: false }),
      ],
      content: fromTelegramHtml(content),
      onUpdate({ editor }) {
        isEmpty = editor.isEmpty;
        onchange?.(editor.getHTML());
      },
    });
    editorInstance = editor;
    isEmpty = editor.isEmpty;
    return {
      destroy() {
        editor.destroy();
        editorInstance = undefined;
      },
    };
  }

  const bold = () => editorInstance?.chain().focus().toggleBold().run();
  const italic = () => editorInstance?.chain().focus().toggleItalic().run();
  const underline = () => editorInstance?.chain().focus().toggleUnderline().run();
  const strike = () => editorInstance?.chain().focus().toggleStrike().run();
  const code = () => editorInstance?.chain().focus().toggleCode().run();
  const blockquote = () => editorInstance?.chain().focus().toggleBlockquote().run();
  const codeBlock = () => editorInstance?.chain().focus().toggleCodeBlock().run();
  const setLink = () => {
    const prev = editorInstance?.getAttributes('link').href ?? '';
    const url = window.prompt('Enter URL (leave empty to remove link):', prev);
    if (url === null) return; // cancelled
    if (url === '') {
      editorInstance?.chain().focus().unsetLink().run();
    } else {
      editorInstance?.chain().focus().setLink({ href: url }).run();
    }
  };
  const clear = () =>
    editorInstance?.chain().focus().clearNodes().unsetAllMarks().run();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="editor-wrap" onclick={() => editorInstance?.commands.focus()}>
  <div class="toolbar" role="toolbar" aria-label="Text formatting">
    <button type="button" onclick={bold} title="Bold"><b>B</b></button>
    <button type="button" onclick={italic} title="Italic"><i>I</i></button>
    <button type="button" onclick={underline} title="Underline"><u>U</u></button>
    <button type="button" onclick={strike} title="Strikethrough"><s>S</s></button>
    <span class="sep"></span>
    <button type="button" onclick={code} title="Inline code">&lt;/&gt;</button>
    <button type="button" onclick={codeBlock} title="Code block">```</button>
    <span class="sep"></span>
    <button type="button" onclick={blockquote} title="Blockquote">❝</button>
    <button type="button" onclick={setLink} title="Link">Link</button>
    <span class="sep"></span>
    <button type="button" onclick={clear} title="Clear formatting" class="clear">✕ Clear</button>
  </div>
  <div class="editor-body">
    {#if isEmpty}
      <div class="placeholder-text">{placeholder}</div>
    {/if}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div use:mountEditor class="editor-content" role="textbox"></div>
  </div>
</div>

<style>
  .editor-wrap {
    border: 1px solid #ccc;
    border-radius: 4px;
    background: white;
    cursor: text;
  }
  .editor-wrap:focus-within {
    border-color: #1a1a2e;
    box-shadow: 0 0 0 2px rgba(26, 26, 46, 0.08);
  }
  .toolbar {
    display: flex;
    gap: 0.2rem;
    padding: 0.4rem 0.5rem;
    border-bottom: 1px solid #ddd;
    flex-wrap: wrap;
    align-items: center;
    background: #f9f9f9;
    cursor: default;
  }
  .toolbar button {
    background: white;
    color: #333;
    border: 1px solid #ccc;
    border-radius: 3px;
    padding: 0.2rem 0.5rem;
    cursor: pointer;
    font-size: 0.85rem;
    min-width: 28px;
    line-height: 1.4;
  }
  .toolbar button:hover {
    background: #e8e8e8;
  }
  .toolbar button.clear {
    margin-left: auto;
    color: #888;
    font-size: 0.8rem;
  }
  .sep {
    width: 1px;
    height: 18px;
    background: #ddd;
    margin: 0 0.15rem;
  }
  .editor-body {
    position: relative;
  }
  .placeholder-text {
    position: absolute;
    top: 0.6rem;
    left: 0.75rem;
    color: #bbb;
    pointer-events: none;
    font-size: 0.9rem;
    line-height: 1.5;
    user-select: none;
  }
  .editor-content {
    position: relative;
    padding: 0.6rem 0.75rem;
    min-height: 180px;
    outline: none;
    font-size: 0.9rem;
    line-height: 1.5;
  }
  :global(.editor-content p) {
    margin: 0 0 0.4em 0;
  }
  :global(.editor-content p:last-child) {
    margin-bottom: 0;
  }
  :global(.editor-content blockquote) {
    border-left: 3px solid #ccc;
    margin: 0.4em 0;
    padding-left: 0.75em;
    color: #555;
  }
  :global(.editor-content code) {
    background: #f0f0f0;
    padding: 0.1em 0.3em;
    border-radius: 3px;
    font-size: 0.88em;
    font-family: monospace;
  }
  :global(.editor-content pre) {
    background: #f0f0f0;
    padding: 0.6em 0.75em;
    border-radius: 4px;
    overflow-x: auto;
    margin: 0.4em 0;
  }
  :global(.editor-content pre code) {
    background: none;
    padding: 0;
  }
  :global(.editor-content a) {
    color: #1a1a2e;
    text-decoration: underline;
  }
</style>
