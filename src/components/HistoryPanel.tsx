import { Component, createSignal, For, Show, onMount, createEffect } from "solid-js";
import {
  listHistory,
  searchHistory,
  deleteHistory,
  togglePin,
  clearHistory,
  writeHistoryItemToClipboard,
  type ClipboardHistoryItem,
} from "../lib/ipc";
import { t } from "../lib/i18n";

interface HistoryPanelProps {
  onBack?: () => void;
  onLoadToPanel?: (content: string) => void;
}

const HistoryPanel: Component<HistoryPanelProps> = (props) => {
  const [items, setItems] = createSignal<ClipboardHistoryItem[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [keyword, setKeyword] = createSignal("");
  const [typeFilter, setTypeFilter] = createSignal<string | undefined>(undefined);
  const [page, setPage] = createSignal(0);
  const [hasMore, setHasMore] = createSignal(true);
  const [copiedId, setCopiedId] = createSignal<number | null>(null);

  const PAGE_SIZE = 30;

  const fetchItems = async (reset = false) => {
    setLoading(true);
    try {
      const offset = reset ? 0 : page() * PAGE_SIZE;
      let result: ClipboardHistoryItem[];

      if (keyword().trim()) {
        result = await searchHistory(keyword().trim(), typeFilter(), PAGE_SIZE);
        setHasMore(false);
      } else {
        result = await listHistory(PAGE_SIZE, offset);
        setHasMore(result.length === PAGE_SIZE);
      }

      if (reset) {
        setItems(result);
        setPage(1);
      } else {
        setItems((prev) => [...prev, ...result]);
        setPage((p) => p + 1);
      }
    } catch (e) {
      console.error("Failed to load history:", e);
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    fetchItems(true);
  });

  // 当搜索关键词或类型过滤变化时重新搜索
  createEffect(() => {
    keyword();
    typeFilter();
    fetchItems(true);
  });

  const handleDelete = async (id: number) => {
    try {
      await deleteHistory(id);
      setItems((prev) => prev.filter((i) => i.id !== id));
    } catch (e) {
      console.error("Failed to delete:", e);
    }
  };

  const handleTogglePin = async (id: number) => {
    try {
      const newState = await togglePin(id);
      setItems((prev) =>
        prev.map((i) => (i.id === id ? { ...i, is_pinned: newState } : i))
      );
    } catch (e) {
      console.error("Failed to toggle pin:", e);
    }
  };

  const handleCopy = async (item: ClipboardHistoryItem) => {
    try {
      await writeHistoryItemToClipboard(item);
      setCopiedId(item.id);
      setTimeout(() => setCopiedId(null), 1500);
    } catch (e) {
      console.error("Failed to copy:", e);
    }
  };

  const handleClearAll = async () => {
    try {
      await clearHistory();
      fetchItems(true);
    } catch (e) {
      console.error("Failed to clear:", e);
    }
  };

  const handleLoadToPanel = (item: ClipboardHistoryItem) => {
    if (item.content && props.onLoadToPanel) {
      props.onLoadToPanel(item.content);
    }
  };

  const contentTypeKeys = [
    { value: undefined as string | undefined, key: "all" },
    { value: "Json", key: "Json" },
    { value: "Yaml", key: "Yaml" },
    { value: "Url", key: "Url" },
    { value: "Code", key: "Code" },
    { value: "MathExpression", key: "MathExpression" },
    { value: "FileList", key: "FileList" },
    { value: "PlainText", key: "PlainText" },
  ];

  const typeColor = (ct: string): string => {
    const colors: Record<string, string> = {
      Json: "bg-amber-400/10 text-amber-600 [html[data-theme=dark]_&]:text-amber-400/80",
      Yaml: "bg-orange-400/10 text-orange-600 [html[data-theme=dark]_&]:text-orange-400/80",
      Url: "bg-sky-400/10 text-sky-600 [html[data-theme=dark]_&]:text-sky-400/80",
      Email: "bg-cyan-400/10 text-cyan-600 [html[data-theme=dark]_&]:text-cyan-400/80",
      PhoneNumber: "bg-emerald-400/10 text-emerald-600 [html[data-theme=dark]_&]:text-emerald-400/80",
      IdCard: "bg-rose-400/10 text-rose-600 [html[data-theme=dark]_&]:text-rose-400/80",
      MathExpression: "bg-violet-400/10 text-violet-600 [html[data-theme=dark]_&]:text-violet-400/80",
      FileList: "bg-indigo-400/10 text-indigo-600 [html[data-theme=dark]_&]:text-indigo-400/80",
      PlainText: "bg-[var(--cb-bg-hover)] text-[var(--cb-text-3)]",
    };
    // content_type 可能是 "Code(\"javascript\")" 格式
    const base = ct.startsWith("Code") ? "Code" : ct;
    return colors[base] ?? "bg-[var(--cb-bg-hover)] text-[var(--cb-text-3)]";
  };

  const typeLabel = (ct: string): string => {
    if (ct.startsWith("Code")) {
      const match = ct.match(/Code\("(.+)"\)/);
      return match ? `${t("contentType.Code")} (${match[1]})` : t("contentType.Code");
    }
    if (ct.startsWith("TableData")) return t("contentType.TableData");
    return t(`contentType.${ct}`) || ct;
  };

  const makePreview = (content: string | null, maxLen: number = 120): string => {
    if (!content) return t("common.noContent");
    const trimmed = content.trim();
    if (trimmed.length <= maxLen) return trimmed;
    return trimmed.slice(0, maxLen) + "…";
  };

  return (
    <div class="flex flex-col h-full bg-transparent text-[var(--cb-text)]">
      {/* 头部 */}
      <header class="flex items-center justify-between px-4 py-3 border-b border-[var(--cb-border)]">
        <div class="flex items-center gap-2">
          <button
            class="text-[var(--cb-text-3)] hover:text-[var(--cb-text)] transition-colors"
            onClick={() => props.onBack?.()}
            title={t("common.back")}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
            </svg>
          </button>
          <h1 class="text-[14px] font-semibold text-[var(--cb-text-2)]">{t("history.historyRecord")}</h1>
        </div>
        <button
          class="text-[13px] text-[var(--cb-text-3)] hover:text-[var(--cb-red-text)] transition-colors"
          onClick={handleClearAll}
          title={t("history.clearUnpinned")}
        >
          {t("history.clear")}
        </button>
      </header>

      {/* 搜索栏 */}
      <div class="px-4 py-2 space-y-2 border-b border-[var(--cb-border-light)]">
        <div class="relative">
          <svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-[var(--cb-text-4)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <input
            type="text"
            placeholder={t("search.placeholder")}
            class="w-full pl-8 pr-3 py-1.5 text-[13px] bg-[var(--cb-bg-input)] border border-[var(--cb-border)] rounded-lg text-[var(--cb-text)] placeholder-[var(--cb-text-4)] focus:outline-none focus:border-[var(--cb-blue-text)]"
            value={keyword()}
            onInput={(e) => setKeyword(e.currentTarget.value)}
          />
        </div>
        <div class="flex gap-1.5 overflow-x-auto">
          <For each={contentTypeKeys}>
            {(ct) => (
              <button
                class={`px-2.5 py-0.5 text-[12px] rounded-full whitespace-nowrap transition-colors ${
                  typeFilter() === ct.value
                    ? "bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)]"
                    : "bg-[var(--cb-bg-input)] text-[var(--cb-text-3)] hover:bg-[var(--cb-bg-hover)] hover:text-[var(--cb-text-2)]"
                }`}
                onClick={() => setTypeFilter(ct.value)}
              >
                {t(`contentType.${ct.key}`)}
              </button>
            )}
          </For>
        </div>
      </div>

      {/* 历史列表 */}
      <div class="flex-1 overflow-y-auto">
        <Show
          when={items().length > 0}
          fallback={
            <div class="flex flex-col items-center justify-center py-16 text-[var(--cb-text-3)] space-y-2">
              <svg class="w-12 h-12 opacity-20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <p class="text-[14px]">{t("history.noHistory")}</p>
            </div>
          }
        >
          <div class="divide-y divide-[var(--cb-border-light)]">
            <For each={items()}>
              {(item) => (
                <div class="group px-4 py-3 hover:bg-[var(--cb-bg-hover)] transition-colors">
                  {/* 内容预览 + 右侧标签 */}
                  <div class="flex items-start gap-2">
                    <p class="flex-1 min-w-0 text-[13px] text-[var(--cb-text-2)] font-mono leading-relaxed break-all line-clamp-3">
                      {makePreview(item.content)}
                    </p>
                    <div class="flex flex-col items-end gap-1 shrink-0 pt-0.5">
                      <span class={`px-1.5 py-0.5 rounded text-[10px] font-medium whitespace-nowrap ${typeColor(item.content_type)}`}>
                        {typeLabel(item.content_type)}
                      </span>
                      <Show when={item.is_sensitive}>
                        <span class="text-[10px] text-[var(--cb-red-text)]">{t("history.sensitive")}</span>
                      </Show>
                    </div>
                  </div>

                  {/* 底部：时间 + 收藏 */}
                  <div class="flex items-center mt-1.5">
                    <span class="text-[11px] text-[var(--cb-text-4)]">{item.created_at}</span>
                    <button
                      class={`ml-auto transition-colors ${
                        item.is_pinned
                          ? "text-[var(--cb-amber-text)]"
                          : "opacity-0 group-hover:opacity-100 text-[var(--cb-text-4)] hover:text-[var(--cb-amber-text)]"
                      }`}
                      onClick={() => handleTogglePin(item.id)}
                      title={item.is_pinned ? t("history.unpin") : t("history.pin")}
                    >
                      <svg class="w-3.5 h-3.5" fill={item.is_pinned ? "currentColor" : "none"} stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" />
                      </svg>
                    </button>
                  </div>

                  {/* 操作按钮 */}
                  <div class="flex items-center gap-2 mt-2 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                      class="text-[11px] px-2 py-0.5 rounded bg-[var(--cb-bg-input)] text-[var(--cb-text-2)] hover:bg-[var(--cb-bg-hover)] transition-colors"
                      onClick={() => handleCopy(item)}
                    >
                      {copiedId() === item.id ? t("common.copied") : t("history.copy")}
                    </button>
                    <button
                      class="text-[11px] px-2 py-0.5 rounded bg-[var(--cb-bg-input)] text-[var(--cb-text-2)] hover:bg-[var(--cb-blue-bg)] hover:text-[var(--cb-blue-text)] transition-colors"
                      onClick={() => handleLoadToPanel(item)}
                    >
                      {t("history.loadToPanel")}
                    </button>
                    <button
                      class="text-[11px] px-2 py-0.5 rounded bg-[var(--cb-bg-input)] text-[var(--cb-text-2)] hover:bg-[var(--cb-red-bg)] hover:text-[var(--cb-red-text)] transition-colors"
                      onClick={() => handleDelete(item.id)}
                    >
                      {t("common.delete")}
                    </button>
                  </div>
                </div>
              )}
            </For>
          </div>

          {/* 加载更多 */}
          <Show when={hasMore() && !keyword().trim()}>
            <div class="flex justify-center py-4">
              <button
                class="text-[13px] text-[var(--cb-text-3)] hover:text-[var(--cb-text-2)] transition-colors disabled:opacity-40"
                onClick={() => fetchItems(false)}
                disabled={loading()}
              >
                {loading() ? t("history.loadingMore") : t("history.loadMore")}
              </button>
            </div>
          </Show>
        </Show>
      </div>
    </div>
  );
};

export default HistoryPanel;
