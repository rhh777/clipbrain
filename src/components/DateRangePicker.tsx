import { Component, createSignal, For, Show } from "solid-js";
import { t } from "../lib/i18n";

/** 日期预设键 */
export type DatePresetKey = "all" | "today" | "yesterday" | "last7days" | "last30days" | "last90days" | "custom";

interface DateRangePickerProps {
  preset: string;
  onSelectPreset: (preset: DatePresetKey) => void;
  onApplyCustomRange: (from: string, to: string) => void;
  onClose: () => void;
}

/** 将 Date 格式化为 "YYYY-MM-DD" */
const fmtDate = (d: Date): string => {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
};

/** 获取某月的天数 */
const daysInMonth = (year: number, month: number): number =>
  new Date(year, month + 1, 0).getDate();

/** 获取某月第一天是星期几 (0=日, 1=一, ..., 6=六) → 转为周一起始 (0=一, ..., 6=日) */
const firstDayOfMonth = (year: number, month: number): number => {
  const day = new Date(year, month, 1).getDay();
  return day === 0 ? 6 : day - 1;
};

const WEEKDAYS = ["一", "二", "三", "四", "五", "六", "日"];
const WEEKDAYS_EN = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

const DateRangePicker: Component<DateRangePickerProps> = (props) => {
  const [showAdvanced, setShowAdvanced] = createSignal(props.preset === "custom");

  const now = new Date();
  const [calYear, setCalYear] = createSignal(now.getFullYear());
  const [calMonth, setCalMonth] = createSignal(now.getMonth());

  const [rangeStart, setRangeStart] = createSignal<string | null>(null);
  const [rangeEnd, setRangeEnd] = createSignal<string | null>(null);

  const presets: DatePresetKey[] = ["all", "today", "yesterday", "last7days", "last30days", "last90days"];

  const isZh = () => {
    try {
      return t("dateFilter.today") === "今天";
    } catch {
      return false;
    }
  };

  const weekdays = () => isZh() ? WEEKDAYS : WEEKDAYS_EN;

  const monthLabel = () => {
    const d = new Date(calYear(), calMonth(), 1);
    if (isZh()) {
      return `${d.getFullYear()} 年 ${d.getMonth() + 1} 月`;
    }
    return d.toLocaleDateString("en-US", { year: "numeric", month: "long" });
  };

  const prevMonth = () => {
    if (calMonth() === 0) {
      setCalYear(calYear() - 1);
      setCalMonth(11);
    } else {
      setCalMonth(calMonth() - 1);
    }
  };

  const nextMonth = () => {
    if (calMonth() === 11) {
      setCalYear(calYear() + 1);
      setCalMonth(0);
    } else {
      setCalMonth(calMonth() + 1);
    }
  };

  /** 生成日历格子数据 */
  const calendarDays = () => {
    const year = calYear();
    const month = calMonth();
    const totalDays = daysInMonth(year, month);
    const startDay = firstDayOfMonth(year, month);
    const cells: (string | null)[] = [];

    // 前置空白
    for (let i = 0; i < startDay; i++) cells.push(null);
    // 当月天数
    for (let d = 1; d <= totalDays; d++) {
      const pad = (n: number) => String(n).padStart(2, "0");
      cells.push(`${year}-${pad(month + 1)}-${pad(d)}`);
    }
    return cells;
  };

  const handleDayClick = (dateStr: string) => {
    const start = rangeStart();
    const end = rangeEnd();

    if (!start || (start && end)) {
      // 开始新的选择
      setRangeStart(dateStr);
      setRangeEnd(null);
    } else {
      // 设置结束日期
      if (dateStr < start) {
        setRangeEnd(start);
        setRangeStart(dateStr);
      } else {
        setRangeEnd(dateStr);
      }
    }
  };

  const isInRange = (dateStr: string): boolean => {
    const start = rangeStart();
    const end = rangeEnd();
    if (!start || !end) return false;
    return dateStr >= start && dateStr <= end;
  };

  const isStart = (dateStr: string): boolean => rangeStart() === dateStr;
  const isEnd = (dateStr: string): boolean => rangeEnd() === dateStr;
  const isToday = (dateStr: string): boolean => fmtDate(new Date()) === dateStr;

  const handleApply = () => {
    const start = rangeStart();
    const end = rangeEnd() || start;
    if (!start) return;
    const fromStr = `${start} 00:00:00`;
    const toStr = `${end} 23:59:59`;
    props.onApplyCustomRange(fromStr, toStr);
    props.onClose();
  };

  return (
    <>
      {/* 背景遮罩 */}
      <div class="fixed inset-0 z-40" onClick={props.onClose} />
      <div class="absolute right-0 top-full mt-1 z-50 rounded-xl border border-[var(--cb-border)] bg-[var(--cb-bg)] shadow-lg animate-fade-in overflow-hidden"
        style={{ width: showAdvanced() ? "280px" : "160px" }}
      >
        {/* 预设列表 */}
        <div class="py-1">
          <For each={presets}>
            {(preset) => (
              <button
                class={`w-full text-left px-3 py-1.5 text-[13px] transition-all ${
                  props.preset === preset
                    ? "bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)]"
                    : "text-[var(--cb-text-2)] hover:bg-[var(--cb-bg-hover)]"
                }`}
                onClick={() => {
                  props.onSelectPreset(preset);
                  props.onClose();
                }}
              >
                {t(`dateFilter.${preset}`)}
              </button>
            )}
          </For>
        </div>

        {/* 分隔线 + 高级选项入口 */}
        <div class="border-t border-[var(--cb-border)]">
          <button
            class={`w-full text-left px-3 py-1.5 text-[13px] transition-all flex items-center justify-between ${
              props.preset === "custom" || showAdvanced()
                ? "text-[var(--cb-blue-text)]"
                : "text-[var(--cb-text-3)] hover:text-[var(--cb-text-2)] hover:bg-[var(--cb-bg-hover)]"
            }`}
            onClick={() => setShowAdvanced(!showAdvanced())}
          >
            <span class="flex items-center gap-1.5">
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
              </svg>
              {t("dateFilter.customRange")}
            </span>
            <svg
              class={`w-3 h-3 transition-transform ${showAdvanced() ? "rotate-180" : ""}`}
              fill="none" stroke="currentColor" viewBox="0 0 24 24"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
            </svg>
          </button>
        </div>

        {/* 展开的日历面板 */}
        <Show when={showAdvanced()}>
          <div class="border-t border-[var(--cb-border)] p-3">
            {/* 月份导航 */}
            <div class="flex items-center justify-between mb-2">
              <button
                class="p-0.5 rounded hover:bg-[var(--cb-bg-hover)] text-[var(--cb-text-3)] hover:text-[var(--cb-text-2)] transition-all"
                onClick={prevMonth}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
                </svg>
              </button>
              <span class="text-[12px] font-medium text-[var(--cb-text-2)]">{monthLabel()}</span>
              <button
                class="p-0.5 rounded hover:bg-[var(--cb-bg-hover)] text-[var(--cb-text-3)] hover:text-[var(--cb-text-2)] transition-all"
                onClick={nextMonth}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                </svg>
              </button>
            </div>

            {/* 星期标题 */}
            <div class="grid grid-cols-7 mb-1">
              <For each={weekdays()}>
                {(wd) => (
                  <div class="text-center text-[10px] text-[var(--cb-text-4)] py-0.5">{wd}</div>
                )}
              </For>
            </div>

            {/* 日期格子 */}
            <div class="grid grid-cols-7">
              <For each={calendarDays()}>
                {(cell) => (
                  <Show when={cell} fallback={<div />}>
                    {(dateStr) => {
                      const day = () => parseInt(dateStr().split("-")[2], 10);
                      return (
                        <button
                          class={`h-7 text-[11px] rounded transition-all relative ${
                            isStart(dateStr()) || isEnd(dateStr())
                              ? "bg-[var(--cb-blue-text)] text-white font-medium"
                              : isInRange(dateStr())
                                ? "bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)]"
                                : isToday(dateStr())
                                  ? "ring-1 ring-[var(--cb-blue-text)]/40 text-[var(--cb-blue-text)] hover:bg-[var(--cb-bg-hover)]"
                                  : "text-[var(--cb-text-2)] hover:bg-[var(--cb-bg-hover)]"
                          }`}
                          onClick={() => handleDayClick(dateStr())}
                        >
                          {day()}
                        </button>
                      );
                    }}
                  </Show>
                )}
              </For>
            </div>

            {/* 选中范围提示 + 确认按钮 */}
            <div class="mt-2 flex items-center justify-between gap-2">
              <div class="text-[11px] text-[var(--cb-text-3)] truncate">
                <Show when={rangeStart()} fallback={<span>{t("dateFilter.selectHint")}</span>}>
                  <span>{rangeStart()}</span>
                  <Show when={rangeEnd()}>
                    <span> ~ {rangeEnd()}</span>
                  </Show>
                </Show>
              </div>
              <button
                class={`px-2.5 py-1 text-[11px] rounded-lg font-medium transition-all shrink-0 ${
                  rangeStart()
                    ? "bg-[var(--cb-blue-text)] text-white hover:opacity-90"
                    : "bg-[var(--cb-bg-hover)] text-[var(--cb-text-4)] cursor-not-allowed"
                }`}
                disabled={!rangeStart()}
                onClick={handleApply}
              >
                {t("dateFilter.apply")}
              </button>
            </div>
          </div>
        </Show>
      </div>
    </>
  );
};

export default DateRangePicker;
