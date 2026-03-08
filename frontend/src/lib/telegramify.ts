/**
 * Converts Tiptap HTML output to Telegram Bot API HTML format.
 *
 * Telegram HTML parse mode supports: <b>, <strong>, <i>, <em>, <u>, <s>, <del>,
 * <code>, <pre>, <a href>, <blockquote> — but NOT <p>, <div>, <span>, or heading tags.
 *
 * Tiptap wraps content in <p> tags and uses <pre><code> for code blocks.
 * This function normalises those differences so the stored value is ready to
 * pass directly to Telegram with ParseMode::Html.
 */
export function toTelegramHtml(html: string): string {
  if (!html) return '';

  return html
    // Code blocks: <pre><code class="...">...</code></pre> → <pre>...</pre>
    .replace(/<pre><code(?:[^>]*)>([\s\S]*?)<\/code><\/pre>/g, '<pre>$1</pre>')
    // Strip <p> tags inside <blockquote>
    .replace(/<blockquote>\s*<p>([\s\S]*?)<\/p>\s*<\/blockquote>/g, '<blockquote>$1</blockquote>')
    // Paragraph end → newline, strip paragraph open
    .replace(/<\/p>/g, '\n')
    .replace(/<p>/g, '')
    // Hard breaks
    .replace(/<br\s*\/?>/g, '\n')
    // Remove any remaining unsupported wrappers
    .replace(/<\/?(?:div|span)[^>]*>/g, '')
    // Collapse 3+ consecutive newlines to 2
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

/**
 * Converts stored Telegram HTML back to Tiptap-compatible HTML for loading
 * into the editor. Wraps newline-separated content in <p> tags.
 */
export function fromTelegramHtml(html: string): string {
  if (!html) return '<p></p>';

  // Keep <pre> blocks intact; split the rest on newlines into <p> tags
  const parts = html.split(/(<pre>[\s\S]*?<\/pre>)/);
  return parts
    .map((part) => {
      if (part.startsWith('<pre>')) return part;
      return part
        .split('\n')
        .map((line) => `<p>${line || '<br/>'}</p>`)
        .join('');
    })
    .join('');
}
