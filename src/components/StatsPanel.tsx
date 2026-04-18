import { Component, createSignal, For, Show, onMount, onCleanup } from "solid-js";
import { getStats, type ActionStats } from "../lib/ipc";
import { t, locale } from "../lib/i18n";

interface StatsPanelProps {
  onBack: () => void;
}

const HelpTip: Component<{ text: string }> = (props) => {
  const [show, setShow] = createSignal(false);
  let timeout: ReturnType<typeof setTimeout> | undefined;

  const open = () => { clearTimeout(timeout); setShow(true); };
  const close = () => { timeout = setTimeout(() => setShow(false), 150); };

  onCleanup(() => clearTimeout(timeout));

  return (
    <span class="relative inline-flex items-center ml-1" onMouseEnter={open} onMouseLeave={close}>
      <svg class="w-3 h-3 text-[var(--cb-text-4)] hover:text-[var(--cb-text-3)] cursor-help transition-colors" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="10" />
        <path stroke-linecap="round" d="M9.5 9.5a2.5 2.5 0 0 1 4.9.5c0 1.5-2.4 2-2.4 2" />
        <circle cx="12" cy="16.5" r="0.5" fill="currentColor" stroke="none" />
      </svg>
      <Show when={show()}>
        <div class="absolute z-50 bottom-full left-1/2 -translate-x-1/2 mb-2 w-max max-w-[180px]">
          <div class="px-2.5 py-1.5 rounded-md text-[11px] leading-[1.5] bg-[rgba(0,0,0,0.75)] text-white/90 shadow-md">
            {props.text}
          </div>
          <div class="w-0 h-0 mx-auto border-x-[5px] border-x-transparent border-t-[5px] border-t-[rgba(0,0,0,0.75)]" />
        </div>
      </Show>
    </span>
  );
};

const StatsPanel: Component<StatsPanelProps> = (props) => {
  const [stats, setStats] = createSignal<ActionStats | null>(null);
  const [loading, setLoading] = createSignal(true);

  onMount(async () => {
    try {
      const data = await getStats(locale());
      setStats(data);
    } catch (e) {
      console.error("Failed to load stats:", e);
    } finally {
      setLoading(false);
    }
  });

  const formatDuration = (ms: number): string => {
    if (ms < 1000) return `${ms}ms`;
    const secs = ms / 1000;
    if (secs < 60) return `${secs.toFixed(1)} ${t("stats.seconds")}`;
    const mins = secs / 60;
    if (mins < 60) return `${mins.toFixed(1)} ${t("stats.minutes")}`;
    const hrs = mins / 60;
    return `${hrs.toFixed(1)} ${t("stats.hours")}`;
  };

  const maxCount = () => {
    const s = stats();
    if (!s || s.top_actions.length === 0) return 1;
    return s.top_actions[0].count;
  };

  const maxDailyCount = () => {
    const s = stats();
    if (!s || s.daily_trend.length === 0) return 1;
    return Math.max(...s.daily_trend.map((d) => d.count));
  };

  return (
    <div class="flex flex-col h-full bg-transparent text-[var(--cb-text)]">
      {/* 头部 */}
      <header class="flex items-center gap-3 px-4 py-2.5 border-b border-[var(--cb-border)]">
        <button
          class="p-1 rounded-lg text-[var(--cb-text-3)] hover:text-[var(--cb-text)] hover:bg-[var(--cb-bg-hover)] transition-all"
          onClick={props.onBack}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <h1 class="text-[14px] font-medium text-[var(--cb-text-2)]">{t("stats.title")}</h1>
      </header>

      {/* 内容区 */}
      <div class="flex-1 overflow-y-auto p-4 space-y-5">
        <Show when={loading()}>
          <div class="flex items-center justify-center py-16 text-[var(--cb-text-3)]">
            <div class="w-5 h-5 border-2 border-[var(--cb-text-3)] border-t-transparent rounded-full animate-spin" />
          </div>
        </Show>

        <Show when={!loading() && stats()}>
          {(s) => (
            <>
              {/* 总览卡片 */}
              <div class="grid grid-cols-2 gap-3">
                <div class="p-4 bg-[var(--cb-bg-card)] border border-[var(--cb-border)] rounded-xl text-center">
                  <p class="text-[28px] font-bold text-[var(--cb-blue-text)]">{s().total_count}</p>
                  <p class="text-[12px] text-[var(--cb-text-3)] mt-1 inline-flex items-center">{t("stats.totalActions")}<HelpTip text={t("stats.totalActionsDesc")} /></p>
                </div>
                <div class="p-4 bg-[var(--cb-bg-card)] border border-[var(--cb-border)] rounded-xl text-center">
                  <p class="text-[28px] font-bold text-[var(--cb-emerald-text)]">{formatDuration(s().total_duration_ms)}</p>
                  <p class="text-[12px] text-[var(--cb-text-3)] mt-1 inline-flex items-center">{t("stats.timeSaved")}<HelpTip text={t("stats.timeSavedDesc")} /></p>
                </div>
              </div>

              {/* 最常用操作 */}
              <Show when={s().top_actions.length > 0}>
                <section class="space-y-2">
                  <h2 class="text-[12px] font-medium text-[var(--cb-text-3)] tracking-wider inline-flex items-center">{t("stats.topActions")}<HelpTip text={t("stats.topActionsDesc")} /></h2>
                  <div class="space-y-1.5">
                    <For each={s().top_actions}>
                      {(action) => (
                        <div class="flex items-center gap-3 p-2.5 bg-[var(--cb-bg-card)] border border-[var(--cb-border)] rounded-xl">
                          <div class="flex-1 min-w-0">
                            <div class="flex items-center justify-between mb-1">
                              <span class="text-[13px] font-medium text-[var(--cb-text-2)] truncate">{action.display_name}</span>
                              <span class="text-[12px] text-[var(--cb-text-3)] shrink-0 ml-2">
                                {action.count} {t("stats.times")}
                              </span>
                            </div>
                            {/* 进度条 */}
                            <div class="h-1.5 bg-[var(--cb-bg-hover)] rounded-full overflow-hidden">
                              <div
                                class="h-full bg-[var(--cb-blue-text)] rounded-full transition-all"
                                style={{ width: `${(action.count / maxCount()) * 100}%` }}
                              />
                            </div>
                          </div>
                        </div>
                      )}
                    </For>
                  </div>
                </section>
              </Show>

              {/* 每日趋势 */}
              <Show when={s().daily_trend.length > 0}>
                <section class="space-y-2">
                  <h2 class="text-[12px] font-medium text-[var(--cb-text-3)] tracking-wider inline-flex items-center">{t("stats.dailyTrend")}<HelpTip text={t("stats.dailyTrendDesc")} /></h2>
                  <div class="p-3 bg-[var(--cb-bg-card)] border border-[var(--cb-border)] rounded-xl">
                    <div class="flex items-end gap-[2px] h-[80px]">
                      <For each={s().daily_trend}>
                        {(day) => (
                          <div
                            class="flex-1 bg-[var(--cb-blue-text)] rounded-t opacity-70 hover:opacity-100 transition-opacity min-w-[4px]"
                            style={{ height: `${Math.max((day.count / maxDailyCount()) * 100, 4)}%` }}
                            title={`${day.date}: ${day.count} ${t("stats.times")}`}
                          />
                        )}
                      </For>
                    </div>
                    <div class="flex justify-between mt-1.5">
                      <span class="text-[10px] text-[var(--cb-text-4)]">
                        {s().daily_trend.length > 0 ? s().daily_trend[0].date.slice(5) : ""}
                      </span>
                      <span class="text-[10px] text-[var(--cb-text-4)]">
                        {s().daily_trend.length > 0 ? s().daily_trend[s().daily_trend.length - 1].date.slice(5) : ""}
                      </span>
                    </div>
                  </div>
                </section>
              </Show>

              {/* 空状态 */}
              <Show when={s().total_count === 0}>
                <div class="flex flex-col items-center justify-center py-12 text-[var(--cb-text-3)] space-y-2">
                  <svg class="w-12 h-12 opacity-20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
                  </svg>
                  <p class="text-[13px]">{t("stats.noData")}</p>
                </div>
              </Show>
            </>
          )}
        </Show>
      </div>
    </div>
  );
};

export default StatsPanel;
