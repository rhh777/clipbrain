import { Component, createEffect, createMemo, createSignal, For, Show, onCleanup, onMount } from "solid-js";
import { getFilePreview, readImageBase64, type ClipboardHistoryItem, type FilePreview } from "../lib/ipc";
import type { ActionDescriptor, ActionOutput } from "../types/clipboard";
import TagEditor from "./TagEditor";
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

const formatTime = (dateStr: string): string => {
  try {
    const d = new Date(dateStr);
    return d.toLocaleString("zh-CN", {
      month: "numeric", day: "numeric",
      hour: "2-digit", minute: "2-digit",
    });
  } catch {
    return dateStr;
  }
};

const groupActions = (actions: ActionDescriptor[]) => ({
  general: actions.filter((action) => action.action_scope === "general"),
  specific: actions.filter((action) => action.action_scope !== "general"),
});

interface DetailPanelProps {
  item: ClipboardHistoryItem | null;
  actions: ActionDescriptor[];
  executing: string | null;
  result: ActionOutput | null;
  error: string | null;
  copied: boolean;
  tags: string[];
  focusArea: "left" | "right";
  streaming: boolean;
  streamContent: string;
  streamThinking: string;
  thinking: boolean;
  onAction: (action: ActionDescriptor) => void;
  onCopyResult: () => void;
  onCopyItem: (item: ClipboardHistoryItem) => void;
  onFocusCustomAction: () => void;
  onTogglePin: (id: number) => void;
  onDelete: (id: number) => void;
  onAddTag: (tagName: string) => void;
  onRemoveTag: (tagName: string) => void;
  onToggleThinking: () => void;
  onCustomAction: (prompt: string) => void;
  clearInputToken: number;
}

const detailImageCache = new Map<string, string | null>();
const filePreviewCache = new Map<string, FilePreview | null>();

const ImagePreview: Component<{ imagePath: string }> = (props) => {
  const [dataUrl, setDataUrl] = createSignal<string | null>(null);

  createEffect(() => {
    const path = props.imagePath;
    if (!path) return;
    const cached = detailImageCache.get(path);
    if (cached !== undefined) { setDataUrl(cached); return; }
    readImageBase64(path)
      .then((url) => { detailImageCache.set(path, url); setDataUrl(url); })
      .catch(() => { detailImageCache.set(path, null); });
  });

  return (
    <Show when={dataUrl()}>
      {(url) => (
        <img src={url()} alt="clipboard image" class="max-w-full max-h-full rounded-xl object-contain" draggable={false} />
      )}
    </Show>
  );
};

const parseFileList = (content: string | null): string[] =>
  content
    ? content
        .split(/\r?\n/)
        .map((path) => path.trim())
        .filter(Boolean)
    : [];

const MAX_FILE_METADATA_ITEMS = 20;
const MAX_TEXT_PREVIEW_LINES = 200;
const MAX_TEXT_PREVIEW_CHARS = 12000;

const getCollapsedTextPreview = (content: string | null) => {
  if (!content) {
    return {
      display: "",
      truncated: false,
      hiddenLines: 0,
    };
  }

  let lineCount = 1;
  let cutIndex = content.length;

  for (let i = 0; i < content.length; i += 1) {
    if (content.charCodeAt(i) === 10) {
      lineCount += 1;
      if (lineCount > MAX_TEXT_PREVIEW_LINES) {
        cutIndex = i;
        break;
      }
    }

    if (i + 1 >= MAX_TEXT_PREVIEW_CHARS) {
      cutIndex = Math.min(cutIndex, i + 1);
      break;
    }
  }

  const truncated = cutIndex < content.length;
  const visible = truncated ? content.slice(0, cutIndex) : content;
  const visibleLines = visible === "" ? 0 : visible.split("\n").length;
  const totalLines = content === "" ? 0 : content.split("\n").length;

  return {
    display: visible,
    truncated,
    hiddenLines: Math.max(totalLines - visibleLines, 0),
  };
};

const formatFileExtension = (preview: FilePreview): string =>
  preview.is_dir ? "DIR" : (preview.extension?.toUpperCase() || "FILE");

const FilePreviewCard: Component<{ path: string }> = (props) => {
  const [preview, setPreview] = createSignal<FilePreview | null>(null);

  createEffect(() => {
    const path = props.path;
    const cached = filePreviewCache.get(path);
    if (cached !== undefined) {
      setPreview(cached);
      return;
    }

    getFilePreview(path)
      .then((result) => {
        filePreviewCache.set(path, result);
        setPreview(result);
      })
      .catch(() => {
        filePreviewCache.set(path, null);
        setPreview(null);
      });
  });

  return (
    <div class="overflow-hidden rounded-2xl border border-[var(--cb-border)] bg-[var(--cb-bg-card)] shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]">
      <div class="border-b border-[var(--cb-border)] px-4 py-3">
        <div class="flex items-center justify-between gap-3">
          <div class="min-w-0">
            <div class="truncate text-[13px] font-medium text-[var(--cb-text)]">
              {preview()?.file_name || props.path.split("/").pop() || props.path}
            </div>
            <div class="truncate text-[11px] text-[var(--cb-text-4)]">{props.path}</div>
          </div>
          <span class="shrink-0 rounded-lg bg-[var(--cb-bg-hover)] px-2 py-1 text-[10px] font-semibold text-[var(--cb-text-3)]">
            {preview() ? formatFileExtension(preview()!) : "FILE"}
          </span>
        </div>
      </div>

      <div class="p-4">
        <Show
          when={preview()}
          fallback={
            <div class="flex h-36 items-center justify-center rounded-xl bg-[var(--cb-bg-hover)] text-[12px] text-[var(--cb-text-4)]">
              {t("common.loading")}
            </div>
          }
        >
          {(filePreview) => (
            <Show
              when={filePreview().kind === "text"}
              fallback={
                <div class="flex h-36 flex-col items-center justify-center gap-3 rounded-xl bg-[var(--cb-bg-hover)]">
                  <Show
                    when={filePreview().data_url}
                    fallback={
                      <div class="flex h-16 w-16 items-center justify-center rounded-2xl bg-white/60 text-[var(--cb-text-3)] shadow-sm [html[data-theme=dark]_&]:bg-white/8">
                        <svg class="h-8 w-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.7" d="M7 3h7l5 5v13a1 1 0 01-1 1H7a2 2 0 01-2-2V5a2 2 0 012-2z" />
                        </svg>
                      </div>
                    }
                  >
                    {(dataUrl) => (
                      <img
                        src={dataUrl()}
                        alt={filePreview().file_name}
                        class="h-20 max-w-full rounded-2xl object-contain"
                        draggable={false}
                      />
                    )}
                  </Show>
                  <div class="text-[12px] text-[var(--cb-text-3)]">
                    {filePreview().kind === "image" ? t("detail.filePreviewImage") : t("detail.filePreviewIcon")}
                  </div>
                </div>
              }
            >
              <div class="overflow-hidden rounded-xl bg-[var(--cb-bg-hover)]">
                <pre class="cb-allow-select px-4 py-3 text-[12px] text-[var(--cb-text-2)] whitespace-pre-wrap break-all font-mono leading-relaxed cursor-text">
                  {filePreview().text || t("common.noContent")}
                </pre>
                <Show when={filePreview().truncated}>
                  <div class="border-t border-[var(--cb-border)] px-4 py-2 text-[11px] text-[var(--cb-text-4)]">
                    {t("detail.filePreviewTruncated")}
                  </div>
                </Show>
              </div>
            </Show>
          )}
        </Show>
      </div>
    </div>
  );
};

const DetailPanel: Component<DetailPanelProps> = (props) => {
  const [customPrompt, setCustomPrompt] = createSignal("");
  const [thinkingExpanded, setThinkingExpanded] = createSignal(false);
  const [contentExpanded, setContentExpanded] = createSignal(false);
  let quickActionsDetails: HTMLDetailsElement | undefined;

  createEffect(() => {
    props.clearInputToken;
    setCustomPrompt("");
  });

  createEffect(() => {
    props.item?.id;
    setContentExpanded(false);
    quickActionsDetails?.removeAttribute("open");
  });

  onMount(() => {
    const handlePointerDown = (event: PointerEvent) => {
      const details = quickActionsDetails;
      const target = event.target;
      if (!(details && details.open && target instanceof Node)) return;
      if (!details.contains(target)) {
        details.open = false;
      }
    };

    document.addEventListener("pointerdown", handlePointerDown);
    onCleanup(() => {
      document.removeEventListener("pointerdown", handlePointerDown);
    });
  });

  const collapsedTextPreview = createMemo(() => getCollapsedTextPreview(props.item?.content ?? null));
  const groupedActions = createMemo(() => groupActions(props.actions));

  const handleCustomSubmit = () => {
    const prompt = customPrompt().trim();
    if (!prompt) return;
    props.onCustomAction(prompt);
    setCustomPrompt("");
  };

  const handleQuickAction = (action: ActionDescriptor) => {
    quickActionsDetails?.removeAttribute("open");
    props.onAction(action);
  };

  const renderActionButtons = (actions: ActionDescriptor[], startIndex = 0) => (
    <For each={actions}>
      {(action, index) => (
        <button
          class="flex items-center justify-between w-full px-3 py-2.5 bg-[var(--cb-bg-card)] hover:bg-[var(--cb-bg-hover)] border border-[var(--cb-border)] rounded-xl transition-all text-left group disabled:opacity-40"
          onClick={() => handleQuickAction(action)}
          disabled={props.executing !== null}
        >
          <div class="min-w-0">
            <div class="text-[13px] font-medium text-[var(--cb-text-2)] group-hover:text-[var(--cb-text)] truncate">
              {action.display_name}
            </div>
            <div class="text-[11px] text-[var(--cb-text-3)] truncate">{action.description}</div>
          </div>
          <div class="flex items-center gap-2 shrink-0 ml-2">
            <Show when={props.executing === action.id}>
              <div class="w-3.5 h-3.5 border-2 border-[var(--cb-blue-text)] border-t-transparent rounded-full animate-spin" />
            </Show>
            <span class="text-[11px] text-[var(--cb-text-4)] font-mono">⌘{startIndex + index() + 1}</span>
          </div>
        </button>
      )}
    </For>
  );

  return (
    <div class="flex-1 flex flex-col overflow-hidden min-w-0">
      <Show
        when={props.item}
        fallback={
          <div data-panel-drag-region class="flex-1 flex flex-col items-center justify-center text-[var(--cb-text-4)] space-y-3">
            <svg class="w-12 h-12 opacity-30" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
            </svg>
            <p class="text-[13px]">{t("detail.emptyTitle")}</p>
            <p class="text-[12px] text-[var(--cb-text-4)]">{t("detail.emptySubtitle")}</p>
          </div>
        }
      >
        {(item) => (
          <div class="flex flex-col h-full overflow-hidden">
            {/* 顶部：类型 + 时间 + 工具栏 */}
              <div data-panel-drag-region class="flex items-center justify-between px-5 py-4 border-b border-[var(--cb-border)] shrink-0 bg-white/20 [html[data-theme=dark]_&]:bg-white/3">
              <div class="flex flex-1 min-w-0 items-center gap-2 flex-wrap">
                <span class={`px-2 py-0.5 rounded-lg text-[12px] font-medium ${typeColor(item().content_type)}`}>
                  {typeLabel(item().content_type)}
                </span>
                <span class="text-[12px] text-[var(--cb-text-3)]">{formatTime(item().created_at)}</span>
                <Show when={item().char_count}>
                  <span class="text-[11px] text-[var(--cb-text-4)]">{t("history.charCount", { n: item().char_count! })}</span>
                </Show>
                <TagEditor
                  tags={props.tags}
                  onAdd={props.onAddTag}
                  onRemove={props.onRemoveTag}
                  compact
                />
              </div>
              <div class="flex items-center gap-1">
                <button
                  class="p-1 rounded-md text-[var(--cb-text-3)] hover:text-[var(--cb-text-2)] hover:bg-[var(--cb-bg-hover)] transition-all"
                  onClick={() => props.onCopyItem(item())}
                  title={t("history.copyOriginal")}
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                  </svg>
                </button>
                <button
                  class={`p-1 rounded-md transition-all ${
                    item().is_pinned
                      ? "text-[var(--cb-amber-text)]"
                      : "text-[var(--cb-text-3)] hover:text-[var(--cb-amber-text)] hover:bg-[var(--cb-bg-hover)]"
                  }`}
                  onClick={() => props.onTogglePin(item().id)}
                  title={item().is_pinned ? t("history.unpin") : t("history.pin")}
                >
                  <svg class="w-4 h-4" fill={item().is_pinned ? "currentColor" : "none"} stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" />
                  </svg>
                </button>
                <button
                  class="p-1 rounded-md text-[var(--cb-text-3)] hover:text-[var(--cb-red-text)] hover:bg-[var(--cb-bg-hover)] transition-all"
                  onClick={() => props.onDelete(item().id)}
                  title={t("common.delete")}
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            </div>

            {/* 主内容区 */}
              <div data-panel-drag-region class="flex-1 min-h-0 px-5 py-4 flex flex-col gap-3 overflow-hidden">
              <div class="flex-1 min-h-0 flex flex-col gap-3 overflow-hidden">
                {/* 完整内容预览 */}
                <Show
                  when={item().content_type === "FileList"}
                  fallback={
                    <Show when={item().image_path} fallback={
                      <div data-no-panel-drag class="flex-1 min-h-[180px] overflow-hidden rounded-2xl border border-[var(--cb-border)] bg-[var(--cb-bg-card)] shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]">
                        <pre class="cb-allow-select h-full min-h-0 overflow-y-auto px-4 py-3 text-[13px] text-[var(--cb-text-2)] whitespace-pre-wrap break-all font-mono leading-relaxed cursor-text">
                          {contentExpanded() || !collapsedTextPreview().truncated
                            ? (item().content ?? t("common.noContent"))
                            : collapsedTextPreview().display}
                        </pre>
                        <Show when={collapsedTextPreview().truncated}>
                          <div class="flex items-center justify-between gap-3 border-t border-[var(--cb-border)] px-4 py-3 text-[12px] text-[var(--cb-text-4)]">
                            <span>
                              {contentExpanded()
                                ? t("detail.longTextExpanded")
                                : t("detail.longTextTruncated", { n: collapsedTextPreview().hiddenLines })}
                            </span>
                            <button
                              class="rounded-lg bg-[var(--cb-blue-bg)] px-3 py-1.5 text-[12px] text-[var(--cb-blue-text)] transition-all hover:opacity-80"
                              onClick={() => setContentExpanded((value) => !value)}
                            >
                              {contentExpanded() ? t("detail.collapseContent") : t("detail.expandContent")}
                            </button>
                          </div>
                        </Show>
                      </div>
                    }>
                      {(imgPath) => (
                        <div data-no-panel-drag class="flex-1 min-h-[180px] overflow-hidden rounded-2xl border border-[var(--cb-border)] bg-[var(--cb-bg-card)] p-4 flex items-center justify-center shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]">
                          <ImagePreview imagePath={imgPath()} />
                        </div>
                      )}
                    </Show>
                  }
                >
                  <div data-no-panel-drag class="flex-1 min-h-[180px] overflow-y-auto rounded-2xl border border-[var(--cb-border)] bg-[var(--cb-bg-card)] p-3 shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]">
                    <div class="mb-3 flex items-center justify-between gap-3 px-1">
                      <h3 class="text-[12px] font-medium text-[var(--cb-text-3)] tracking-wider">
                        {t("detail.filePreviewTitle")}
                      </h3>
                      <span class="text-[11px] text-[var(--cb-text-4)]">
                        {t("detail.filePreviewCount", { n: parseFileList(item().content).length })}
                      </span>
                    </div>
                    <div class="grid gap-3">
                      <Show when={parseFileList(item().content)[0]}>
                        {(path) => <FilePreviewCard path={path()} />}
                      </Show>
                    </div>
                    <Show when={parseFileList(item().content).length > 1}>
                      <div class="rounded-2xl border border-[var(--cb-border)] bg-[var(--cb-bg-hover)]/60 p-3">
                        <div class="mb-2 text-[12px] font-medium text-[var(--cb-text-3)]">
                          {t("detail.filePreviewCount", { n: parseFileList(item().content).length - 1 })}
                        </div>
                        <div class="space-y-2">
                          <For each={parseFileList(item().content).slice(1, MAX_FILE_METADATA_ITEMS + 1)}>
                            {(path) => {
                              const fileName = () => path.split("/").pop() || path;
                              return (
                                <div class="rounded-xl bg-[var(--cb-bg-card)] px-3 py-2">
                                  <div class="truncate text-[12px] font-medium text-[var(--cb-text)]">{fileName()}</div>
                                  <div class="truncate text-[11px] text-[var(--cb-text-4)]">{path}</div>
                                </div>
                              );
                            }}
                          </For>
                        </div>
                        <Show when={parseFileList(item().content).length - 1 > MAX_FILE_METADATA_ITEMS}>
                          <div class="px-1 pt-3 text-[11px] text-[var(--cb-text-4)]">
                            {t("detail.filePreviewMore", { n: parseFileList(item().content).length - 1 - MAX_FILE_METADATA_ITEMS })}
                          </div>
                        </Show>
                      </div>
                    </Show>
                  </div>
                </Show>

              {/* 操作结果（流式 / 最终） */}
              <Show when={props.result || props.streaming}>
                <div data-no-panel-drag class="shrink-0 space-y-1.5 animate-fade-in">
                  <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2">
                      <h3 class="text-[12px] font-medium text-[var(--cb-text-3)] tracking-wider">{t("detail.actionResult")}</h3>
                      <Show when={props.streaming && !props.streamContent}>
                        <span class="inline-flex items-center gap-1 text-[12px] text-[var(--cb-blue-text)]">
                          <span class="w-1.5 h-1.5 rounded-full bg-[var(--cb-blue-text)] animate-pulse" />
                          {t("clipboard.thinking")}
                        </span>
                      </Show>
                      <Show when={props.streaming && props.streamContent}>
                        <span class="inline-flex items-center gap-1 text-[12px] text-[var(--cb-blue-text)]">
                          <span class="w-1.5 h-1.5 rounded-full bg-[var(--cb-blue-text)] animate-pulse" />
                          {t("clipboard.generating")}
                        </span>
                      </Show>
                    </div>
                    <Show when={!props.streaming || props.streamContent}>
                      <button
                        class="text-[12px] px-2.5 py-1 rounded-lg bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)] hover:opacity-80 transition-all"
                        onClick={props.onCopyResult}
                      >
                        {props.copied ? t("common.copied") : t("detail.copyResult")}
                      </button>
                    </Show>
                  </div>

                  {/* 思考过程 */}
                  <Show when={props.streamThinking}>
                    {(() => {
                      const isThinking = () => props.streaming && !props.streamContent;
                      return (
                        <div class="bg-[var(--cb-bg-card)] rounded-xl p-3 border border-[var(--cb-border)]">
                          <button
                            class="flex items-center gap-1.5 w-full text-left"
                            onClick={() => !isThinking() && setThinkingExpanded((v) => !v)}
                            style={{ cursor: isThinking() ? "default" : "pointer" }}
                          >
                            <svg class="w-3.5 h-3.5 text-[var(--cb-text-3)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                            </svg>
                            <span class="text-[12px] font-medium text-[var(--cb-text-3)]">{t("clipboard.thinkingProcess")}</span>
                            <Show when={!isThinking()}>
                              <svg class={`w-3 h-3 text-[var(--cb-text-4)] transition-transform ${thinkingExpanded() ? "rotate-90" : ""}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                              </svg>
                            </Show>
                          </button>
                          <Show when={isThinking()}>
                            <pre class="cb-allow-select text-[12px] text-[var(--cb-text-3)] whitespace-nowrap overflow-hidden text-ellipsis font-mono leading-relaxed mt-1.5 cursor-text">
                              {props.streamThinking.split("\n").filter(Boolean).pop() || ""}
                            </pre>
                          </Show>
                          <Show when={!isThinking() && thinkingExpanded()}>
                            <pre class="cb-allow-select text-[12px] text-[var(--cb-text-3)] whitespace-pre-wrap break-all font-mono leading-relaxed max-h-[120px] overflow-y-auto mt-1.5 cursor-text">
                              {props.streamThinking}
                            </pre>
                          </Show>
                        </div>
                      );
                    })()}
                  </Show>

                  {/* 正文结果 */}
                    <div class="bg-[var(--cb-emerald-bg)] rounded-xl p-3 border border-[var(--cb-emerald-text)]/10">
                      <pre class="cb-allow-select text-[13px] text-[var(--cb-emerald-text)] whitespace-pre-wrap break-all font-mono leading-relaxed max-h-[180px] overflow-y-auto cursor-text">
                        {props.result?.result || props.streamContent || ""}
                      </pre>
                    <Show when={props.streaming && !props.streamContent}>
                      <div class="flex items-center gap-1 mt-2">
                        <span class="w-1 h-1 rounded-full bg-[var(--cb-emerald-text)] animate-bounce" style="animation-delay: 0ms" />
                        <span class="w-1 h-1 rounded-full bg-[var(--cb-emerald-text)] animate-bounce" style="animation-delay: 150ms" />
                        <span class="w-1 h-1 rounded-full bg-[var(--cb-emerald-text)] animate-bounce" style="animation-delay: 300ms" />
                      </div>
                    </Show>
                  </div>
                </div>
              </Show>

              {/* 错误信息 */}
              <Show when={props.error}>
                {(err) => (
                    <div data-no-panel-drag class="shrink-0 bg-[var(--cb-red-bg)] border border-[var(--cb-red-text)]/10 rounded-xl p-3 animate-fade-in">
                      <p class="text-[13px] text-[var(--cb-red-text)]">{err()}</p>
                    </div>
                  )}
                </Show>

              </div>

               {/* 自定义操作 */}
               <Show when={props.item?.content && props.item?.content_type !== "FileList"}>
                 <div data-no-panel-drag class="shrink-0 flex items-end gap-2">
                   <div class="relative flex-1 min-w-0">
                     <input
                       data-custom-action-input
                       type="text"
                       autocomplete="off"
                       autocorrect="off"
                       autocapitalize="off"
                       spellcheck={false}
                       class="w-full rounded-2xl border border-[var(--cb-border)] bg-[var(--cb-bg-elevated)]/90 py-3 pl-4 pr-44 text-[13px] text-[var(--cb-text)] placeholder-[var(--cb-text-4)] shadow-[0_18px_40px_rgba(15,23,42,0.08)] backdrop-blur-md focus:outline-none focus:border-[var(--cb-blue-text)] transition-all"
                       placeholder={t("detail.customPlaceholder")}
                       value={customPrompt()}
                       onInput={(e) => setCustomPrompt(e.currentTarget.value)}
                       onFocus={props.onFocusCustomAction}
                       on:keydown={(e) => { if (e.key === "Enter" && !e.isComposing) { e.preventDefault(); e.stopPropagation(); handleCustomSubmit(); } }}
                       disabled={props.executing !== null}
                     />
                     <div class="absolute inset-y-0 right-2 flex items-center gap-1">
                       <Show when={props.actions.some((a) => a.requires_model)}>
                         <button
                           class={`inline-flex h-8 items-center rounded-xl px-2 text-[11px] font-medium transition-all ${
                             props.thinking
                               ? "bg-[var(--cb-purple-bg)] text-[var(--cb-purple-text)]"
                               : "bg-[var(--cb-bg-input)] text-[var(--cb-text-3)] hover:text-[var(--cb-text-2)]"
                           }`}
                           onClick={props.onToggleThinking}
                           title={props.thinking ? t("detail.thinkingTooltipOn") : t("detail.thinkingTooltipOff")}
                         >
                           {props.thinking ? t("detail.thinkingOn") : t("detail.thinkingOff")}
                         </button>
                       </Show>
                       <button
                         class="inline-flex h-8 w-8 items-center justify-center rounded-xl bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)] hover:opacity-80 transition-all disabled:opacity-40"
                         onClick={handleCustomSubmit}
                         disabled={props.executing !== null || !customPrompt().trim()}
                         title={t("detail.customSend")}
                       >
                         <Show
                           when={props.executing === "custom_prompt"}
                           fallback={
                             <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                               <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h12m0 0-5-5m5 5-5 5" />
                             </svg>
                           }
                         >
                           <div class="w-3.5 h-3.5 border-2 border-[var(--cb-blue-text)] border-t-transparent rounded-full animate-spin" />
                         </Show>
                       </button>
                     </div>
                   </div>
                    <Show when={props.actions.length > 0}>
                      <details
                        ref={quickActionsDetails}
                        class="group relative shrink-0"
                     >
                       <summary class="inline-flex h-[46px] items-center gap-1.5 rounded-2xl border border-[var(--cb-border)] bg-[var(--cb-bg-elevated)]/90 px-3 text-[12px] font-medium text-[var(--cb-text-2)] shadow-[0_18px_40px_rgba(15,23,42,0.08)] backdrop-blur-md transition-all cursor-pointer select-none list-none hover:border-[var(--cb-border-strong)] hover:bg-[var(--cb-bg-hover)] [&::-webkit-details-marker]:hidden">
                         {t("detail.moreActions")}
                         <svg class="w-3.5 h-3.5 transition-transform group-open:-rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                           <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 15l-7-7-7 7" />
                         </svg>
                        </summary>
                        <div class="absolute bottom-full right-0 z-20 mb-2 w-[280px] rounded-2xl border border-[var(--cb-border)] bg-[var(--cb-bg-elevated)]/95 p-2 shadow-[0_24px_60px_rgba(15,23,42,0.16)] backdrop-blur-xl">
                          <div class="grid max-h-[240px] gap-2 overflow-y-auto pr-1">
                            <Show when={groupedActions().specific.length > 0}>
                              <section class="space-y-1.5">
                                <div class="px-1 text-[10px] font-semibold uppercase tracking-wider text-[var(--cb-text-4)]">
                                  {t("common.specificActions")}
                                </div>
                                {renderActionButtons(groupedActions().specific)}
                              </section>
                            </Show>
                            <Show when={groupedActions().general.length > 0}>
                              <section class="space-y-1.5">
                                <div class="px-1 text-[10px] font-semibold uppercase tracking-wider text-[var(--cb-text-4)]">
                                  {t("common.generalActions")}
                                </div>
                                {renderActionButtons(groupedActions().general, groupedActions().specific.length)}
                              </section>
                            </Show>
                          </div>
                        </div>
                      </details>
                    </Show>
                 </div>
               </Show>
              </div>
          </div>
        )}
      </Show>
    </div>
  );
};

export default DetailPanel;
