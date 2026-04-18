import { createSignal, createRoot } from "solid-js";
import zhCN from "../locales/zh-CN.json";
import enUS from "../locales/en-US.json";

export type Locale = "zh-CN" | "en-US";

const locales: Record<Locale, Record<string, any>> = {
  "zh-CN": zhCN,
  "en-US": enUS,
};

/** 从嵌套对象中按 dot-path 取值：t("settings.title") → locales[locale]["settings"]["title"] */
function getNestedValue(obj: Record<string, any>, path: string): string | undefined {
  const keys = path.split(".");
  let current: any = obj;
  for (const key of keys) {
    if (current == null || typeof current !== "object") return undefined;
    current = current[key];
  }
  return typeof current === "string" ? current : undefined;
}

/** 全局 i18n 状态（Solid reactive） */
const { locale, setLocale, t } = createRoot(() => {
  const [locale, setLocaleSignal] = createSignal<Locale>("zh-CN");

  /** 切换语言并持久化 */
  const setLocale = (l: Locale) => {
    setLocaleSignal(l);
    try {
      localStorage.setItem("clipbrain-locale", l);
    } catch {}
  };

  /** 翻译函数：支持嵌套 key + 插值 */
  const t = (key: string, vars?: Record<string, string | number>): string => {
    const dict = locales[locale()];
    let text = getNestedValue(dict, key);

    // fallback: 尝试从 zh-CN 取
    if (text === undefined && locale() !== "zh-CN") {
      text = getNestedValue(locales["zh-CN"], key);
    }

    // 最终 fallback: 返回 key 本身
    if (text === undefined) return key;

    // 插值替换: "已清除 {{count}} 条记录" → "已清除 5 条记录"
    if (vars) {
      for (const [k, v] of Object.entries(vars)) {
        text = text.replace(new RegExp(`\\{\\{${k}\\}\\}`, "g"), String(v));
      }
    }
    return text;
  };

  // 初始化：从 localStorage 恢复
  try {
    const saved = localStorage.getItem("clipbrain-locale");
    if (saved === "en-US" || saved === "zh-CN") {
      setLocaleSignal(saved);
    }
  } catch {}

  return { locale, setLocale, t };
});

export { locale, setLocale, t };
