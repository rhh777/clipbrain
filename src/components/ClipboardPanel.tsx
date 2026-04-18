import { Component, createSignal, For, Show, onMount, onCleanup } from "solid-js";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { executeAction, executeActionStream, writeToClipboard } from "../lib/ipc";
import type {
  ClipboardChangeEvent,
  ActionDescriptor,
  ActionOutput,
  ActionStreamPayload,
  ContentType,
} from "../types/clipboard";
import { contentTypeLabel, contentTypeColor } from "../types/clipboard";
import { t } from "../lib/i18n";

interface ClipboardPanelProps {
  onOpenSettings?: () => void;
  onOpenHistory?: () => void;
}

const ClipboardPanel: Component<ClipboardPanelProps> = (props) => {
  const [clipEvent, setClipEvent] = createSignal<ClipboardChangeEvent | null>(null);
  const [executing, setExecuting] = createSignal<string | null>(null);
  const [result, setResult] = createSignal<ActionOutput | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [copied, setCopied] = createSignal(false);
  const [streaming, setStreaming] = createSignal(false);
  const [streamContent, setStreamContent] = createSignal("");
  const [streamThinking, setStreamThinking] = createSignal("");
  const [thinkingExpanded, setThinkingExpanded] = createSignal(false);

  let unlisten: UnlistenFn | undefined;

  onMount(async () => {
    unlisten = await listen<ClipboardChangeEvent>("clipboard-change", (event) => {
      setClipEvent(event.payload);
      setResult(null);
      setError(null);
      setCopied(false);
    });
  });

  onCleanup(() => {
    unlisten?.();
  });

  const groupActions = (actions: ActionDescriptor[]) => ({
    general: actions.filter((action) => action.action_scope === "general"),
    specific: actions.filter((action) => action.action_scope !== "general"),
  });

  const handleAction = async (action: ActionDescriptor) => {
    const ev = clipEvent();
    if (!ev) return;

    setExecuting(action.id);
    setResult(null);
    setError(null);
    setCopied(false);
    setStreamContent("");
    setStreamThinking("");
    setThinkingExpanded(false);

    if (action.requires_model) {
      // 流式执行：立即展示结果框 + 流式更新
      setStreaming(true);

      let unlistenStream: UnlistenFn | undefined;
      try {
        unlistenStream = await listen<ActionStreamPayload>("action-stream", (event) => {
          const p = event.payload;
          if (p.action_id !== action.id) return;
          switch (p.event_type) {
            case "thinking":
              setStreamThinking((prev) => prev + p.content);
              break;
            case "delta":
              setStreamContent((prev) => prev + p.content);
              break;
          }
        });

        const output = await executeActionStream(action.id, ev.content, ev.content_type);
        setResult(output);
      } catch (e: any) {
        setError(typeof e === "string" ? e : e?.message ?? t("clipboard.actionFailed"));
      } finally {
        unlistenStream?.();
        setStreaming(false);
        setExecuting(null);
      }
    } else {
      // 非模型操作：直接执行
      try {
        const output = await executeAction(action.id, ev.content, ev.content_type);
        setResult(output);
      } catch (e: any) {
        setError(typeof e === "string" ? e : e?.message ?? t("clipboard.actionFailed"));
      } finally {
        setExecuting(null);
      }
    }
  };

  const handleCopy = async () => {
    const text = result()?.result || streamContent();
    if (!text) return;
    try {
      await writeToClipboard(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // ignore
    }
  };

  return (
    <div class="flex flex-col h-full bg-transparent text-[var(--cb-text)]">
      {/* 头部 */}
      <header class="flex items-center justify-between px-4 py-3 border-b border-[var(--cb-border)]">
        <h1 class="text-[14px] font-semibold text-[var(--cb-text-2)]">ClipBrain</h1>
        <div class="flex items-center gap-2">
          <button
            class="text-[var(--cb-text-3)] hover:text-[var(--cb-text)] transition-colors"
            onClick={() => props.onOpenHistory?.()}
            title={t("history.historyRecord")}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </button>
          <button
            class="text-[var(--cb-text-3)] hover:text-[var(--cb-text)] transition-colors"
            onClick={() => props.onOpenSettings?.()}
            title={t("common.settings")}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>
        </div>
      </header>

      {/* 内容区 */}
      <div class="flex-1 overflow-y-auto p-4 space-y-4">
        <Show
          when={clipEvent()}
          fallback={
            <div class="flex flex-col items-center justify-center flex-1 min-h-[400px] text-[var(--cb-text-3)] space-y-3">
              <svg class="w-16 h-16 opacity-20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 5H7a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
              </svg>
              <p class="text-[14px] font-medium">{t("search.emptyHint")}</p>
              <p class="text-[13px] text-[var(--cb-text-4)]">{t("detail.emptySubtitle")}</p>
              <div class="flex flex-wrap gap-2 mt-2 justify-center">
                <span class="px-2 py-0.5 rounded text-[12px] bg-amber-400/10 text-amber-500">JSON</span>
                <span class="px-2 py-0.5 rounded text-[12px] bg-sky-400/10 text-sky-500">URL</span>
                <span class="px-2 py-0.5 rounded text-[12px] bg-violet-400/10 text-violet-500">{t("contentType.MathExpression")}</span>
                <span class="px-2 py-0.5 rounded text-[12px] bg-green-400/10 text-green-500">{t("contentType.Code")}</span>
                <span class="px-2 py-0.5 rounded text-[12px] bg-[var(--cb-bg-hover)] text-[var(--cb-text-3)]">{t("contentType.PlainText")}</span>
              </div>
            </div>
          }
        >
          {(ev) => (
            <>
              {/* 分类标签 */}
              <div class="flex items-center gap-2">
                <span class={`px-2 py-0.5 rounded text-xs font-medium ${contentTypeColor(ev().content_type)}`}>
                  {contentTypeLabel(ev().content_type)}
                </span>
                <span class="text-[12px] text-[var(--cb-text-3)]">
                  {new Date(ev().timestamp).toLocaleTimeString()}
                </span>
              </div>

              {/* 内容预览 */}
              <div class="bg-[var(--cb-bg-card)] rounded-lg p-3 border border-[var(--cb-border)]">
                <pre class="text-[14px] text-[var(--cb-text-2)] whitespace-pre-wrap break-all font-mono leading-relaxed">
                  {ev().preview}
                </pre>
              </div>

              {/* 可用操作 */}
              <Show when={ev().actions.length > 0}>
                <div class="space-y-2">
                  <h2 class="text-[12px] font-medium text-[var(--cb-text-3)] uppercase tracking-wider">{t("clipboard.availableActions")}</h2>
                  {(() => {
                    const grouped = groupActions(ev().actions);
                    const renderActionList = (actions: ActionDescriptor[]) => (
                      <div class="grid gap-2">
                        <For each={actions}>
                          {(action) => (
                            <button
                              class="flex items-center justify-between w-full px-3 py-2.5 bg-[var(--cb-bg-card)] hover:bg-[var(--cb-bg-hover)] border border-[var(--cb-border)] rounded-lg transition-colors text-left group disabled:opacity-50"
                              onClick={() => handleAction(action)}
                              disabled={executing() !== null}
                            >
                              <div>
                                <div class="text-[14px] font-medium text-[var(--cb-text-2)] group-hover:text-[var(--cb-text)]">
                                  {action.display_name}
                                </div>
                                <div class="text-[12px] text-[var(--cb-text-3)]">{action.description}</div>
                              </div>
                              <Show when={executing() === action.id}>
                                <div class="w-4 h-4 border-2 border-[var(--cb-blue-text)] border-t-transparent rounded-full animate-spin" />
                              </Show>
                            </button>
                          )}
                        </For>
                      </div>
                    );

                    return (
                      <div class="space-y-3">
                        <Show when={grouped.specific.length > 0}>
                          <section class="space-y-2">
                            <div class="text-[11px] font-semibold uppercase tracking-wider text-[var(--cb-text-4)]">
                              {t("common.specificActions")}
                            </div>
                            {renderActionList(grouped.specific)}
                          </section>
                        </Show>
                        <Show when={grouped.general.length > 0}>
                          <section class="space-y-2">
                            <div class="text-[11px] font-semibold uppercase tracking-wider text-[var(--cb-text-4)]">
                              {t("common.generalActions")}
                            </div>
                            {renderActionList(grouped.general)}
                          </section>
                        </Show>
                      </div>
                    );
                  })()}
                </div>
              </Show>

              {/* 执行结果（流式 / 最终） */}
              <Show when={result() || streaming()}>
                <div class="space-y-2">
                  <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2">
                      <h2 class="text-[12px] font-medium text-[var(--cb-text-3)] uppercase tracking-wider">{t("clipboard.result")}</h2>
                      <Show when={streaming() && !streamContent()}>
                        <span class="inline-flex items-center gap-1 text-[12px] text-[var(--cb-blue-text)]">
                          <span class="w-1.5 h-1.5 rounded-full bg-[var(--cb-blue-text)] animate-pulse" />
                          {t("clipboard.thinking")}
                        </span>
                      </Show>
                      <Show when={streaming() && streamContent()}>
                        <span class="inline-flex items-center gap-1 text-[12px] text-[var(--cb-blue-text)]">
                          <span class="w-1.5 h-1.5 rounded-full bg-[var(--cb-blue-text)] animate-pulse" />
                          {t("clipboard.generating")}
                        </span>
                      </Show>
                    </div>
                    <Show when={!streaming() || streamContent()}>
                      <button
                        class="text-[12px] px-2 py-1 rounded bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)] hover:opacity-80 transition-colors"
                        onClick={handleCopy}
                      >
                        {copied() ? t("common.copied") : t("clipboard.copyResult")}
                      </button>
                    </Show>
                  </div>

                  {/* 思考过程 */}
                  <Show when={streamThinking()}>
                    {(() => {
                      const isThinking = () => streaming() && !streamContent();
                      return (
                        <div class="bg-[var(--cb-bg-card)] rounded-lg p-3 border border-[var(--cb-border)]">
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
                            <pre class="text-[12px] text-[var(--cb-text-3)] whitespace-nowrap overflow-hidden text-ellipsis font-mono leading-relaxed mt-1.5">
                              {streamThinking().split("\n").filter(Boolean).pop() || ""}
                            </pre>
                          </Show>
                          <Show when={!isThinking() && thinkingExpanded()}>
                            <pre class="text-[13px] text-[var(--cb-text-3)] whitespace-pre-wrap break-all font-mono leading-relaxed max-h-32 overflow-y-auto mt-1.5">
                              {streamThinking()}
                            </pre>
                          </Show>
                        </div>
                      );
                    })()}
                  </Show>

                  {/* 正文结果 */}
                  <div class="bg-[var(--cb-emerald-bg)] rounded-lg p-3 border border-[var(--cb-emerald-text)]/10">
                    <pre class="text-[14px] text-[var(--cb-emerald-text)] whitespace-pre-wrap break-all font-mono leading-relaxed">
                      {result()?.result || streamContent() || ""}
                    </pre>
                    <Show when={streaming() && !streamContent()}>
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
              <Show when={error()}>
                {(err) => (
                  <div class="bg-[var(--cb-red-bg)] border border-[var(--cb-red-text)]/10 rounded-lg p-3">
                    <p class="text-[14px] text-[var(--cb-red-text)]">{err()}</p>
                  </div>
                )}
              </Show>
            </>
          )}
        </Show>
      </div>
    </div>
  );
};

export default ClipboardPanel;
