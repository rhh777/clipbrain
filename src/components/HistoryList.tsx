import { Component, createEffect, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { getAppIcon, readImageBase64, type ClipboardHistoryItem } from "../lib/ipc";
import { t } from "../lib/i18n";

/** 内容类型 → 显示标签 */
const typeLabel = (ct: string): string => {
  if (ct.startsWith("Code")) {
    const match = ct.match(/Code\("(.+)"\)/);
    return match ? `${t("contentType.Code")} (${match[1]})` : t("contentType.Code");
  }
  if (ct.startsWith("TableData")) return t("contentType.TableData");
  return t(`contentType.${ct}`) || ct;
};

/** 内容类型 → 标签颜色（低饱和度） */
const typeColor = (ct: string): string => {
  const base = ct.startsWith("Code") ? "Code" : ct.startsWith("TableData") ? "TableData" : ct;
  const colors: Record<string, string> = {
    Json: "bg-amber-400/10 text-amber-600 [html[data-theme=dark]_&]:text-amber-400/80",
    Yaml: "bg-orange-400/10 text-orange-600 [html[data-theme=dark]_&]:text-orange-400/80",
    Url: "bg-sky-400/10 text-sky-600 [html[data-theme=dark]_&]:text-sky-400/80",
    Email: "bg-cyan-400/10 text-cyan-600 [html[data-theme=dark]_&]:text-cyan-400/80",
    PhoneNumber: "bg-emerald-400/10 text-emerald-600 [html[data-theme=dark]_&]:text-emerald-400/80",
    IdCard: "bg-rose-400/10 text-rose-600 [html[data-theme=dark]_&]:text-rose-400/80",
    MathExpression: "bg-violet-400/10 text-violet-600 [html[data-theme=dark]_&]:text-violet-400/80",
    Code: "bg-green-400/10 text-green-600 [html[data-theme=dark]_&]:text-green-400/80",
    TableData: "bg-teal-400/10 text-teal-600 [html[data-theme=dark]_&]:text-teal-400/80",
    Image: "bg-pink-400/10 text-pink-600 [html[data-theme=dark]_&]:text-pink-400/80",
    FileList: "bg-indigo-400/10 text-indigo-600 [html[data-theme=dark]_&]:text-indigo-400/80",
    PlainText: "bg-[var(--cb-bg-hover)] text-[var(--cb-text-3)]",
    Unknown: "bg-[var(--cb-bg-hover)] text-[var(--cb-text-3)]",
  };
  return colors[base] ?? colors["Unknown"];
};

const makePreview = (content: string | null, maxLen: number = 100): string => {
  if (!content) return t("common.noContent");
  const trimmed = content.replace(/\n/g, " ").trim();
  if (trimmed.length <= maxLen) return trimmed;
  return trimmed.slice(0, maxLen) + "…";
};

const formatTime = (dateStr: string): string => {
  try {
    const d = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 1) return t("history.justNow");
    if (diffMin < 60) return t("history.minutesAgo", { n: diffMin });
    const diffH = Math.floor(diffMin / 60);
    if (diffH < 24) return t("history.hoursAgo", { n: diffH });
    return d.toLocaleDateString("zh-CN", { month: "numeric", day: "numeric" });
  } catch {
    return dateStr;
  }
};

interface HistoryListProps {
  items: ClipboardHistoryItem[];
  loading: boolean;
  keyword: string;
  selectedId: number | null;
  selectedIndex: number;
  hasMore: boolean;
  expanded?: boolean;
  currentClipContent?: string;
  onSelectItem: (item: ClipboardHistoryItem, index: number) => void;
  onTogglePin: (id: number) => void;
  onLoadMore: () => void;
  onDoubleClick?: (item: ClipboardHistoryItem) => void;
}

/** 应用图标缓存（app name → asset URL） */
const iconCache = new Map<string, string | null>();
const itemMediaSlotClass = "h-8 w-10 shrink-0";

const useVisibility = () => {
  const [visible, setVisible] = createSignal(false);
  let ref: HTMLDivElement | undefined;

  onMount(() => {
    if (typeof IntersectionObserver === "undefined" || !ref) {
      setVisible(true);
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: "120px" }
    );

    observer.observe(ref);
    onCleanup(() => observer.disconnect());
  });

  return { visible, setRef: (el: HTMLDivElement) => (ref = el) };
};

const AppIcon: Component<{ appName: string | null }> = (props) => {
  const [iconUrl, setIconUrl] = createSignal<string | null>(null);
  const { visible, setRef } = useVisibility();

  createEffect(() => {
    const name = props.appName;
    if (!name || !visible()) return;

    const cached = iconCache.get(name);
    if (cached !== undefined) {
      setIconUrl(cached);
      return;
    }

    getAppIcon(name)
      .then((dataUrl) => {
        iconCache.set(name, dataUrl);
        setIconUrl(dataUrl);
      })
      .catch(() => {
        iconCache.set(name, null);
        setIconUrl(null);
      });
  });

  return (
    <div ref={setRef} class="h-8 w-8">
      <Show
        when={iconUrl()}
        fallback={
          <div class="h-8 w-8 rounded-md bg-[var(--cb-bg-hover)] flex items-center justify-center text-[var(--cb-text-4)]">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
            </svg>
          </div>
        }
      >
        {(url) => (
          <img
            src={url()}
            alt={props.appName ?? ""}
            class="h-8 w-8 rounded-md"
            draggable={false}
          />
        )}
      </Show>
    </div>
  );
};

const imageThumbCache = new Map<string, string | null>();

const ImageThumbnail: Component<{ imagePath: string }> = (props) => {
  const [dataUrl, setDataUrl] = createSignal<string | null>(null);
  const { visible, setRef } = useVisibility();

  createEffect(() => {
    const path = props.imagePath;
    if (!visible()) return;
    const cached = imageThumbCache.get(path);
    if (cached !== undefined) {
      setDataUrl(cached);
      return;
    }

    readImageBase64(path)
      .then((url) => {
        imageThumbCache.set(path, url);
        setDataUrl(url);
      })
      .catch(() => {
        imageThumbCache.set(path, null);
        setDataUrl(null);
      });
  });

  return (
    <div ref={setRef} class="h-7 w-10">
      <Show
        when={dataUrl()}
        fallback={
          <div class="h-7 w-10 rounded-md bg-[var(--cb-bg-hover)]" />
        }
      >
        {(url) => (
          <img
            src={url()}
            alt="clipboard image"
            class="h-7 w-10 rounded-md object-cover"
            draggable={false}
          />
        )}
      </Show>
    </div>
  );
};

const HistoryList: Component<HistoryListProps> = (props) => {
  let listRef: HTMLDivElement | undefined;

  // 自动滚动到选中项
  createEffect(() => {
    const idx = props.selectedIndex;
    if (listRef) {
      const el = listRef.querySelector(`[data-index="${idx}"]`);
      el?.scrollIntoView({ block: "nearest" });
    }
  });

  return (
    <div
      class={`flex flex-col border-r border-[var(--cb-border)] bg-white/55 [html[data-theme=dark]_&]:bg-transparent ${
        props.expanded ? "flex-1 min-w-0" : "w-[320px] min-w-[280px] shrink-0"
      }`}
    >
      {/* 列表 */}
      <div ref={listRef} class="flex-1 overflow-y-auto">
        <Show
          when={props.items.length > 0}
          fallback={
            <div class="flex flex-col items-center justify-center py-16 text-[var(--cb-text-3)] space-y-2">
              <svg class="w-10 h-10 opacity-30" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
              </svg>
              <p class="text-[13px]">
                {props.keyword ? t("search.noResults") : t("search.emptyHint")}
              </p>
            </div>
          }
        >
          <div>
            <For each={props.items}>
              {(item, index) => {
                const isSelected = () => props.selectedId === item.id;

                return (
                  <div
                    data-index={index()}
                    class={`group px-3 py-1 cursor-pointer transition-all border-l-2 h-12 flex items-center overflow-hidden ${
                      isSelected()
                        ? "bg-[var(--cb-selection-bg)] border-l-[var(--cb-blue-text)]"
                        : "border-l-transparent hover:bg-[var(--cb-bg-hover)]"
                    }`}
                    onClick={() => props.onSelectItem(item, index())}
                    onDblClick={() => props.onDoubleClick?.(item)}
                    >
                        <div class={`${itemMediaSlotClass} mr-2.5 flex items-center justify-center`}>
                          <Show when={item.image_path} fallback={<AppIcon appName={item.source_app} />}>
                            {(imagePath) => <ImageThumbnail imagePath={imagePath()} />}
                          </Show>
                        </div>

                      <div class="flex-1 min-w-0 overflow-hidden">
                        <p class={`text-[13px] font-mono leading-snug truncate ${isSelected() ? "text-[var(--cb-text)]" : "text-[var(--cb-text-2)]"}`}>
                          {item.image_path ? (item.content?.trim() || t("contentType.Image")) : makePreview(item.content)}
                        </p>
                      </div>

                    <div class="flex flex-col items-end gap-1 shrink-0 ml-2">
                      <span class={`px-1.5 py-0 rounded text-[10px] font-medium whitespace-nowrap ${typeColor(item.content_type)}`}>
                        {typeLabel(item.content_type)}
                      </span>
                      <div class="flex items-center gap-1">
                        <button
                          class={`transition-all shrink-0 ${
                            item.is_pinned
                              ? "text-[var(--cb-amber-text)]"
                              : "opacity-0 group-hover:opacity-100 text-[var(--cb-text-4)] hover:!text-[var(--cb-amber-text)]"
                          }`}
                          onClick={(e) => {
                            e.stopPropagation();
                            props.onTogglePin(item.id);
                          }}
                          title={item.is_pinned ? t("history.unpin") : t("history.pin")}
                        >
                          <svg class="w-3.5 h-3.5" fill={item.is_pinned ? "currentColor" : "none"} stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" />
                          </svg>
                        </button>
                      </div>
                    </div>
                  </div>
                );
              }}
            </For>

            {/* 加载更多 */}
            <Show when={props.hasMore}>
              <div class="flex justify-center py-3">
                <button
                  class="text-[12px] text-[var(--cb-text-3)] hover:text-[var(--cb-text-2)] transition-colors disabled:opacity-40"
                  onClick={props.onLoadMore}
                  disabled={props.loading}
                >
                  {props.loading ? t("history.loadingMore") : t("history.loadMore")}
                </button>
              </div>
            </Show>
          </div>
        </Show>
      </div>
    </div>
  );
};

export default HistoryList;
