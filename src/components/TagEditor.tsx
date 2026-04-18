import { Component, createSignal, For, Show } from "solid-js";
import { t } from "../lib/i18n";

interface TagEditorProps {
  tags: string[];
  onAdd: (tagName: string) => void;
  onRemove: (tagName: string) => void;
  compact?: boolean;
  class?: string;
}

const TagEditor: Component<TagEditorProps> = (props) => {
  const [editing, setEditing] = createSignal(false);
  const [newTag, setNewTag] = createSignal("");

  const handleAdd = () => {
    const name = newTag().trim().replace(/\s/g, "");
    if (!name) return;
    if (props.tags.includes(name)) {
      setNewTag("");
      return;
    }
    props.onAdd(name);
    setNewTag("");
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleAdd();
    } else if (e.key === "Escape") {
      setEditing(false);
      setNewTag("");
    }
  };

  return (
    <div class={`flex items-center gap-1.5 flex-wrap ${props.compact ? "" : "min-h-[28px]"} ${props.class ?? ""}`}>
      <For each={props.tags}>
        {(tag) => (
          <span class="inline-flex items-center gap-0.5 px-2 py-0.5 rounded-lg bg-[var(--cb-purple-bg)] text-[var(--cb-purple-text)] text-[11px]">
            #{tag}
            <button
              class="ml-0.5 opacity-50 hover:opacity-100 transition-opacity"
              onClick={() => props.onRemove(tag)}
              title={t("tags.removeTag")}
            >
              <svg class="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </span>
        )}
      </For>

      <Show
        when={editing()}
        fallback={
          <button
            class={`inline-flex items-center rounded-lg bg-[var(--cb-bg-input)] text-[var(--cb-text-3)] text-[11px] hover:bg-[var(--cb-bg-hover)] hover:text-[var(--cb-text-2)] transition-all ${
              props.compact ? "h-5 w-5 justify-center" : "gap-0.5 px-1.5 py-0.5"
            }`}
            onClick={() => setEditing(true)}
            title={t("tags.addTag")}
          >
            <svg class="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M12 4v16m8-8H4" />
            </svg>
            <Show when={!props.compact}>
              {t("tags.addTag")}
            </Show>
          </button>
        }
      >
        <input
          type="text"
          class={`${props.compact ? "w-16 h-5" : "w-20 py-0.5"} px-1.5 text-[11px] bg-[var(--cb-bg-input)] border border-[var(--cb-border)] rounded-lg text-[var(--cb-text-2)] placeholder-[var(--cb-text-4)] focus:outline-none focus:border-[var(--cb-purple-text)]`}
          placeholder={t("tags.tagPlaceholder")}
          value={newTag()}
          onInput={(e) => setNewTag(e.currentTarget.value)}
          onKeyDown={handleKeyDown}
          onBlur={() => {
            if (!newTag().trim()) setEditing(false);
          }}
          ref={(el) => setTimeout(() => el.focus(), 0)}
        />
      </Show>
    </div>
  );
};

export default TagEditor;
