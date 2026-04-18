import { Component, createSignal, Show, For, onMount, createEffect, onCleanup, type JSX } from "solid-js";
import { setupAndTestModel, type ModelConfigInput, type ModelConfigOutput, listModelConfigs, deleteModelConfig, setActiveModel, getConfig, updateShortcut, clearHistoryWithRetention, countHistoryOverSize, clearHistoryOverSize, saveConfig, listActions, type QuickActionBinding, type AppConfig } from "../lib/ipc";
import { theme, setTheme, type Theme } from "../lib/theme";
import { t, locale, setLocale, type Locale } from "../lib/i18n";
import type { ActionDescriptor, ContentType } from "../types/clipboard";

interface SettingsPageProps {
  onBack: () => void;
}

type QuickActionTypeKey =
  | "Json"
  | "Yaml"
  | "Url"
  | "Code"
  | "MathExpression"
  | "TableData"
  | "PlainText"
  | "Email"
  | "PhoneNumber"
  | "IdCard";

interface QuickActionTypeOption {
  key: QuickActionTypeKey;
  contentType: ContentType;
}

const QUICK_ACTION_TYPE_OPTIONS: QuickActionTypeOption[] = [
  { key: "Json", contentType: { type: "Json" } },
  { key: "Yaml", contentType: { type: "Yaml" } },
  { key: "Url", contentType: { type: "Url" } },
  { key: "Code", contentType: { type: "Code", detail: "text" } },
  { key: "MathExpression", contentType: { type: "MathExpression" } },
  { key: "TableData", contentType: { type: "TableData", detail: "csv" } },
  { key: "PlainText", contentType: { type: "PlainText" } },
  { key: "Email", contentType: { type: "Email" } },
  { key: "PhoneNumber", contentType: { type: "PhoneNumber" } },
  { key: "IdCard", contentType: { type: "IdCard" } },
];

const DEFAULT_QUICK_ACTION_TYPE: QuickActionTypeKey = "PlainText";

const SettingsPage: Component<SettingsPageProps> = (props) => {
  const [name, setName] = createSignal("default");
  const [baseUrl, setBaseUrl] = createSignal("https://api.openai.com/v1");
  const [apiKey, setApiKey] = createSignal("");
  const [showApiKey, setShowApiKey] = createSignal(false);
  const [model, setModel] = createSignal("gpt-4o");
  const [timeoutSecs, setTimeoutSecs] = createSignal(30);
  const [maxTokens, setMaxTokens] = createSignal(2048);

  const [saving, setSaving] = createSignal(false);
  const [message, setMessage] = createSignal<{ type: "success" | "error"; text: string } | null>(null);

  // --- 清空数据状态 ---
  const retentionOptions: { labelKey: string; days: number }[] = [
    { labelKey: "settings.retainDay1", days: 1 },
    { labelKey: "settings.retainWeek1", days: 7 },
    { labelKey: "settings.retainMonth1", days: 30 },
    { labelKey: "settings.retainMonth3", days: 90 },
    { labelKey: "settings.retainMonth6", days: 180 },
    { labelKey: "settings.retainYear1", days: 365 },
    { labelKey: "settings.retainAll", days: 0 },
  ];
  const [selectedRetention, setSelectedRetention] = createSignal(0); // days
  const [confirmStep, setConfirmStep] = createSignal(false);
  const [clearing, setClearing] = createSignal(false);
  const [clearMsg, setClearMsg] = createSignal<{ type: "success" | "error"; text: string } | null>(null);

  const handleClearData = async () => {
    setClearing(true);
    setClearMsg(null);
    try {
      const deleted = await clearHistoryWithRetention(selectedRetention());
      setClearMsg({ type: "success", text: t("settings.clearSuccess", { count: deleted }) });
    } catch (e: any) {
      setClearMsg({ type: "error", text: typeof e === "string" ? e : e?.message ?? t("settings.clearFailed") });
    } finally {
      setClearing(false);
      setConfirmStep(false);
    }
  };

  // --- 按大小清理状态 ---
  type SizeUnit = "KB" | "MB";
  const [sizeThreshold, setSizeThreshold] = createSignal(100); // 阈值数值
  const [sizeUnit, setSizeUnit] = createSignal<SizeUnit>("KB");
  const [sizePreview, setSizePreview] = createSignal<{ count: number; totalBytes: number } | null>(null);
  const [sizeScanning, setSizeScanning] = createSignal(false);
  const [sizeConfirmStep, setSizeConfirmStep] = createSignal(false);
  const [sizeClearing, setSizeClearing] = createSignal(false);
  const [sizeMsg, setSizeMsg] = createSignal<{ type: "success" | "error"; text: string } | null>(null);

  const thresholdBytes = (): number => {
    const n = sizeThreshold();
    if (!Number.isFinite(n) || n <= 0) return 0;
    return Math.floor(n * (sizeUnit() === "MB" ? 1024 * 1024 : 1024));
  };

  const formatBytes = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  };

  const handleScanOverSize = async () => {
    const bytes = thresholdBytes();
    if (bytes <= 0) {
      setSizeMsg({ type: "error", text: t("settings.sizeThresholdInvalid") });
      return;
    }
    setSizeScanning(true);
    setSizeMsg(null);
    setSizeConfirmStep(false);
    try {
      const [count, totalBytes] = await countHistoryOverSize(bytes);
      setSizePreview({ count, totalBytes });
    } catch (e: any) {
      setSizeMsg({ type: "error", text: typeof e === "string" ? e : e?.message ?? t("settings.clearFailed") });
    } finally {
      setSizeScanning(false);
    }
  };

  const handleClearOverSize = async () => {
    const bytes = thresholdBytes();
    if (bytes <= 0) return;
    setSizeClearing(true);
    setSizeMsg(null);
    try {
      const deleted = await clearHistoryOverSize(bytes);
      setSizeMsg({ type: "success", text: t("settings.clearSuccess", { count: deleted }) });
      setSizePreview(null);
    } catch (e: any) {
      setSizeMsg({ type: "error", text: typeof e === "string" ? e : e?.message ?? t("settings.clearFailed") });
    } finally {
      setSizeClearing(false);
      setSizeConfirmStep(false);
    }
  };

  // --- 快捷键状态 ---
  const [currentShortcut, setCurrentShortcut] = createSignal("Alt+CommandOrControl+C");
  const [recording, setRecording] = createSignal(false);
  const [shortcutMsg, setShortcutMsg] = createSignal<{ type: "success" | "error"; text: string } | null>(null);
  const [autoStart, setAutoStart] = createSignal(false);
  const [showDetailPanelByDefault, setShowDetailPanelByDefault] = createSignal(true);
  const [showSearchToolbarButtons, setShowSearchToolbarButtons] = createSignal(false);
  const [clearInputsOnPanelOpen, setClearInputsOnPanelOpen] = createSignal(false);

  // --- 快捷操作状态 ---
  const [quickActions, setQuickActions] = createSignal<QuickActionBinding[]>([]);
  const [qaMsg, setQaMsg] = createSignal<{ type: "success" | "error"; text: string } | null>(null);
  const [recordingQaIndex, setRecordingQaIndex] = createSignal<number | null>(null);
  const [appConfig, setAppConfig] = createSignal<AppConfig | null>(null);
  const [quickActionCatalog, setQuickActionCatalog] = createSignal<Partial<Record<QuickActionTypeKey, ActionDescriptor[]>>>({});
  const [quickActionTypeByIndex, setQuickActionTypeByIndex] = createSignal<Partial<Record<number, QuickActionTypeKey>>>({});

  // --- 已保存的模型配置列表 ---
  const [savedConfigs, setSavedConfigs] = createSignal<ModelConfigOutput[]>([]);
  const [editingConfig, setEditingConfig] = createSignal(false);

  const getQuickActionOptions = (typeKey: QuickActionTypeKey): ActionDescriptor[] =>
    quickActionCatalog()[typeKey] ?? [];

  const groupQuickActionOptions = (typeKey: QuickActionTypeKey) => {
    const options = getQuickActionOptions(typeKey);
    return {
      specific: options.filter((action) => action.action_scope !== "general"),
      general: options.filter((action) => action.action_scope === "general"),
    };
  };

  const findActionDescriptor = (
    actionId: string,
    catalog: Partial<Record<QuickActionTypeKey, ActionDescriptor[]>> = quickActionCatalog()
  ): ActionDescriptor | undefined => {
    for (const option of QUICK_ACTION_TYPE_OPTIONS) {
      const found = catalog[option.key]?.find((action) => action.id === actionId);
      if (found) return found;
    }
    return undefined;
  };

  const findPreferredQuickActionType = (
    actionId: string,
    catalog: Partial<Record<QuickActionTypeKey, ActionDescriptor[]>> = quickActionCatalog()
  ): QuickActionTypeKey => {
    for (const option of QUICK_ACTION_TYPE_OPTIONS) {
      const found = catalog[option.key]?.find((action) => action.id === actionId && action.action_scope !== "general");
      if (found) return option.key;
    }
    for (const option of QUICK_ACTION_TYPE_OPTIONS) {
      const found = catalog[option.key]?.find((action) => action.id === actionId);
      if (found) return option.key;
    }
    return DEFAULT_QUICK_ACTION_TYPE;
  };

  const buildQuickActionTypeMap = (
    actions: QuickActionBinding[],
    catalog: Partial<Record<QuickActionTypeKey, ActionDescriptor[]>>,
    previous: Partial<Record<number, QuickActionTypeKey>> = {}
  ): Partial<Record<number, QuickActionTypeKey>> => {
    const next: Partial<Record<number, QuickActionTypeKey>> = {};
    actions.forEach((action, index) => {
      const preserved = previous[index];
      if (preserved && catalog[preserved]?.some((candidate) => candidate.id === action.action_id)) {
        next[index] = preserved;
        return;
      }
      next[index] = findPreferredQuickActionType(action.action_id, catalog);
    });
    return next;
  };

  const pickDefaultQuickAction = (typeKey: QuickActionTypeKey): ActionDescriptor | undefined => {
    const grouped = groupQuickActionOptions(typeKey);
    return grouped.specific[0] ?? grouped.general[0];
  };

  const refreshQuickActionCatalog = async (currentLocale: string) => {
    const entries = await Promise.all(
      QUICK_ACTION_TYPE_OPTIONS.map(async (option) => [
        option.key,
        await listActions(option.contentType, currentLocale),
      ] as const)
    );
    const catalog = Object.fromEntries(entries) as Partial<Record<QuickActionTypeKey, ActionDescriptor[]>>;
    setQuickActionCatalog(catalog);
    setQuickActionTypeByIndex((prev) => buildQuickActionTypeMap(quickActions(), catalog, prev));
  };

  const resolveQuickActionLabel = (
    binding: QuickActionBinding,
    previousAction: ActionDescriptor | undefined,
    nextAction: ActionDescriptor | undefined
  ): string => {
    if (!nextAction) return binding.label;
    if (!binding.label || (previousAction && binding.label === previousAction.display_name)) {
      return nextAction.display_name;
    }
    return binding.label;
  };

  const refreshModelConfigs = async () => {
    try {
      const configs = await listModelConfigs();
      setSavedConfigs(configs);
    } catch {}
  };

  onMount(async () => {
    try {
      const cfg = await getConfig();
      setCurrentShortcut(cfg.hotkey.open_panel);
      setQuickActions(cfg.hotkey.quick_actions ?? []);
      setAutoStart(cfg.general.auto_start ?? false);
      setShowDetailPanelByDefault(cfg.general.show_detail_panel_by_default ?? true);
      setShowSearchToolbarButtons(cfg.general.show_search_toolbar_buttons ?? false);
      setClearInputsOnPanelOpen(cfg.general.clear_inputs_on_panel_open ?? false);
      setAppConfig(cfg);
    } catch {}
    await refreshModelConfigs();
  });

  createEffect(() => {
    const currentLocale = locale();
    void refreshQuickActionCatalog(currentLocale).catch(() => {});
  });

  const saveAppConfig = async (nextConfig: AppConfig, successText?: string) => {
    try {
      await saveConfig(nextConfig);
      setAppConfig(nextConfig);
      if (successText) {
        setMessage({ type: "success", text: successText });
      }
    } catch (e: any) {
      setMessage({ type: "error", text: typeof e === "string" ? e : e?.message ?? t("settings.saveFailed") });
      throw e;
    }
  };

  const handleAddQuickAction = () => {
    const typeKey = DEFAULT_QUICK_ACTION_TYPE;
    const defaultAction = pickDefaultQuickAction(typeKey);
    setQuickActions((prev) => [
      ...prev,
      {
        label: defaultAction?.display_name ?? "",
        action_id: defaultAction?.id ?? "",
        shortcut: "",
        enabled: true,
      },
    ]);
    setQuickActionTypeByIndex((prev) => ({
      ...prev,
      [quickActions().length]: typeKey,
    }));
  };

  const handleRemoveQuickAction = async (index: number) => {
    const updated = quickActions().filter((_, i) => i !== index);
    setQuickActions(updated);
    setQuickActionTypeByIndex((prev) => buildQuickActionTypeMap(updated, quickActionCatalog(), prev));
    await saveQuickActions(updated);
    setQaMsg({ type: "success", text: t("settings.quickActionDeleted") });
  };

  const handleQaFieldChange = (index: number, field: keyof QuickActionBinding, value: string | boolean) => {
    setQuickActions((prev) =>
      prev.map((qa, i) => (i === index ? { ...qa, [field]: value } : qa))
    );
  };

  const handleQaTypeChange = (index: number, typeKey: QuickActionTypeKey) => {
    const current = quickActions()[index];
    if (!current) return;

    const previousAction = findActionDescriptor(current.action_id);
    const options = getQuickActionOptions(typeKey);
    const nextAction = options.find((action) => action.id === current.action_id) ?? pickDefaultQuickAction(typeKey);

    setQuickActionTypeByIndex((prev) => ({ ...prev, [index]: typeKey }));
    setQuickActions((prev) =>
      prev.map((qa, i) =>
        i === index
          ? {
              ...qa,
              action_id: nextAction?.id ?? "",
              label: resolveQuickActionLabel(qa, previousAction, nextAction),
            }
          : qa
      )
    );
  };

  const handleQaShortcutKeyDown = async (index: number, e: KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") { setRecordingQaIndex(null); return; }
    const sc = eventToShortcutStr(e);
    if (!sc) return;
    const updated = quickActions().map((qa, i) => (i === index ? { ...qa, shortcut: sc } : qa));
    setQuickActions(updated);
    setRecordingQaIndex(null);
    await saveQuickActions(updated);
  };

  const saveQuickActions = async (actions: QuickActionBinding[]) => {
    const cfg = appConfig();
    if (!cfg) return;
    const newCfg = { ...cfg, hotkey: { ...cfg.hotkey, quick_actions: actions } };
    try {
      await saveAppConfig(newCfg);
      setQaMsg({ type: "success", text: t("settings.quickActionSaved") });
    } catch (e: any) {
      setQaMsg({ type: "error", text: typeof e === "string" ? e : e?.message ?? "Save failed" });
    }
  };

  const handleSaveQuickActions = () => saveQuickActions(quickActions());

  /** 将 KeyboardEvent 转为 Tauri 快捷键字符串 */
  const eventToShortcutStr = (e: KeyboardEvent): string | null => {
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return null;

    const parts: string[] = [];
    if (e.metaKey || e.ctrlKey) parts.push("CommandOrControl");
    if (e.shiftKey) parts.push("Shift");
    if (e.altKey) parts.push("Alt");

    if (parts.length === 0) return null; // 必须有修饰键

    // macOS 下按住 Option 键时 e.key 会返回组合字符（如 √ ≈ 等），
    // 需要通过 e.code 获取物理按键来还原真实键名
    let key = e.key;
    if (e.altKey && e.code) {
      if (e.code.startsWith("Key")) {
        key = e.code.slice(3); // "KeyV" → "V"
      } else if (e.code.startsWith("Digit")) {
        key = e.code.slice(5); // "Digit1" → "1"
      } else if (e.code === "Space") {
        key = "Space";
      } else if (e.code.startsWith("Arrow")) {
        key = e.code.slice(5); // "ArrowUp" → "Up"
      } else {
        key = e.code;
      }
    }

    // 规范化键名
    if (key.length === 1) key = key.toUpperCase();
    else if (key === " ") key = "Space";
    else if (key === "ArrowUp") key = "Up";
    else if (key === "ArrowDown") key = "Down";
    else if (key === "ArrowLeft") key = "Left";
    else if (key === "ArrowRight") key = "Right";

    parts.push(key);
    return parts.join("+");
  };

  /** 显示友好的快捷键文本 */
  const formatShortcut = (s: string): string => {
    return s
      .replace(/CommandOrControl/g, navigator.platform.includes("Mac") ? "⌘" : "Ctrl")
      .replace(/Shift/g, "⇧")
      .replace(/Alt/g, navigator.platform.includes("Mac") ? "⌥" : "Alt")
      .replace(/\+/g, " ");
  };

  const handleShortcutKeyDown = async (e: KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();

    if (e.key === "Escape") {
      setRecording(false);
      return;
    }

    const shortcut = eventToShortcutStr(e);
    if (!shortcut) return;

    const old = currentShortcut();
    try {
      await updateShortcut(old, shortcut);
      setCurrentShortcut(shortcut);
      setShortcutMsg({ type: "success", text: t("settings.shortcutUpdated", { shortcut: formatShortcut(shortcut) }) });
    } catch (err: any) {
      setShortcutMsg({ type: "error", text: typeof err === "string" ? err : err?.message ?? t("settings.shortcutFailed") });
    }
    setRecording(false);
  };

  // 全局键盘监听：解决 macOS WebView 中按钮无法接收 keydown 事件的问题
  createEffect(() => {
    if (recording()) {
      const handler = (e: KeyboardEvent) => handleShortcutKeyDown(e);
      document.addEventListener("keydown", handler);
      onCleanup(() => document.removeEventListener("keydown", handler));
    }
  });

  createEffect(() => {
    const idx = recordingQaIndex();
    if (idx !== null) {
      const handler = (e: KeyboardEvent) => handleQaShortcutKeyDown(idx, e);
      document.addEventListener("keydown", handler);
      onCleanup(() => document.removeEventListener("keydown", handler));
    }
  });

  const handleSave = async () => {
    if (!baseUrl().trim() || !model().trim()) {
      setMessage({ type: "error", text: t("settings.fillRequired") });
      return;
    }

    setSaving(true);
    setMessage(null);

    const config: ModelConfigInput = {
      name: name(),
      base_url: baseUrl(),
      api_key: apiKey(),
      model: model(),
      timeout_secs: timeoutSecs(),
      max_tokens: maxTokens(),
    };

    try {
      const result = await setupAndTestModel(config);
      setMessage({ type: "success", text: result });
      await refreshModelConfigs();
      setEditingConfig(false);
    } catch (e: any) {
      setMessage({ type: "error", text: typeof e === "string" ? e : e?.message ?? t("settings.saveFailed") });
    } finally {
      setSaving(false);
    }
  };

  const handleEditConfig = (cfg: ModelConfigOutput) => {
    setName(cfg.name);
    setBaseUrl(cfg.base_url);
    setApiKey(cfg.api_key);
    setShowApiKey(false);
    setModel(cfg.model);
    setTimeoutSecs(cfg.timeout_secs);
    setMaxTokens(cfg.max_tokens);
    setEditingConfig(true);
    setMessage(null);
  };

  const handleNewConfig = () => {
    setName("");
    setBaseUrl("https://api.openai.com/v1");
    setApiKey("");
    setShowApiKey(false);
    setModel("gpt-4o");
    setTimeoutSecs(30);
    setMaxTokens(2048);
    setEditingConfig(true);
    setMessage(null);
  };

  const handleDeleteConfig = async (cfgName: string) => {
    try {
      await deleteModelConfig(cfgName);
      await refreshModelConfigs();
      setMessage({ type: "success", text: t("settings.configDeleted", { name: cfgName }) });
    } catch (e: any) {
      setMessage({ type: "error", text: typeof e === "string" ? e : e?.message ?? t("settings.saveFailed") });
    }
  };

  const handleSetActive = async (cfgName: string) => {
    try {
      await setActiveModel(cfgName);
      await refreshModelConfigs();
    } catch (e: any) {
      setMessage({ type: "error", text: typeof e === "string" ? e : e?.message ?? t("settings.saveFailed") });
    }
  };

  // --- 字体大小状态 (滑块, 12~22px) ---
  const FONT_SIZE_MIN = 12;
  const FONT_SIZE_MAX = 22;
  const FONT_SIZE_DEFAULT = 15;
  const getStoredFontSize = (): number => {
    try { const v = parseInt(localStorage.getItem("clipbrain-font-size-px") ?? "", 10); if (v >= FONT_SIZE_MIN && v <= FONT_SIZE_MAX) return v; } catch {} return FONT_SIZE_DEFAULT;
  };
  const [fontSizePx, setFontSizePx] = createSignal(getStoredFontSize());
  const applyFontSize = (px: number) => {
    setFontSizePx(px);
    document.documentElement.style.fontSize = px + "px";
    try { localStorage.setItem("clipbrain-font-size-px", String(px)); } catch {};
  };
  // 初始化应用字体大小
  onMount(() => { document.documentElement.style.fontSize = fontSizePx() + "px"; });

  const handleToggleSearchToolbarButtons = async (enabled: boolean) => {
    const cfg = appConfig();
    if (!cfg) return;

    setShowSearchToolbarButtons(enabled);
    const nextConfig = {
      ...cfg,
      general: {
        ...cfg.general,
        show_search_toolbar_buttons: enabled,
      },
    };

    try {
      await saveAppConfig(nextConfig);
    } catch {
      setShowSearchToolbarButtons(cfg.general.show_search_toolbar_buttons ?? false);
    }
  };

  const handleToggleDetailPanelByDefault = async (enabled: boolean) => {
    const cfg = appConfig();
    if (!cfg) return;

    setShowDetailPanelByDefault(enabled);
    const nextConfig = {
      ...cfg,
      general: {
        ...cfg.general,
        show_detail_panel_by_default: enabled,
      },
    };

    try {
      await saveAppConfig(nextConfig);
    } catch {
      setShowDetailPanelByDefault(cfg.general.show_detail_panel_by_default ?? true);
    }
  };

  const handleToggleAutoStart = async (enabled: boolean) => {
    const cfg = appConfig();
    if (!cfg) return;

    setAutoStart(enabled);
    const nextConfig = {
      ...cfg,
      general: {
        ...cfg.general,
        auto_start: enabled,
      },
    };

    try {
      await saveAppConfig(nextConfig);
    } catch {
      setAutoStart(cfg.general.auto_start ?? false);
    }
  };

  const handleToggleClearInputsOnPanelOpen = async (enabled: boolean) => {
    const cfg = appConfig();
    if (!cfg) return;

    setClearInputsOnPanelOpen(enabled);
    const nextConfig = {
      ...cfg,
      general: {
        ...cfg.general,
        clear_inputs_on_panel_open: enabled,
      },
    };

    try {
      await saveAppConfig(nextConfig);
    } catch {
      setClearInputsOnPanelOpen(cfg.general.clear_inputs_on_panel_open ?? false);
    }
  };

  const themeOptions: { value: Theme; labelKey: string }[] = [
    { value: "system", labelKey: "settings.themeSystem" },
    { value: "light", labelKey: "settings.themeLight" },
    { value: "dark", labelKey: "settings.themeDark" },
  ];

  const localeOptions: { value: Locale; label: string }[] = [
    { value: "zh-CN", label: "中文" },
    { value: "en-US", label: "English" },
  ];

  const helpSections: { titleKey: string; descKey: string; bullets: string[] }[] = [
    {
      titleKey: "settings.guideBasicsTitle",
      descKey: "settings.guideBasicsDesc",
      bullets: [
        "settings.guideBasicsBullet1",
        "settings.guideBasicsBullet2",
        "settings.guideBasicsBullet3",
      ],
    },
    {
      titleKey: "settings.guideModesTitle",
      descKey: "settings.guideModesDesc",
      bullets: [
        "settings.guideModesBullet1",
        "settings.guideModesBullet2",
        "settings.guideModesBullet3",
      ],
    },
    {
      titleKey: "settings.guideEfficiencyTitle",
      descKey: "settings.guideEfficiencyDesc",
      bullets: [
        "settings.guideEfficiencyBullet1",
        "settings.guideEfficiencyBullet2",
        "settings.guideEfficiencyBullet3",
      ],
    },
  ];

  // --- 设置分类 Tab ---
  type SettingsTab = "guide" | "general" | "shortcuts" | "ai" | "data";
  const [activeTab, setActiveTab] = createSignal<SettingsTab>("general");
  const tabItems: { key: SettingsTab; labelKey: string; icon: string }[] = [
    { key: "guide", labelKey: "settings.categoryGuide", icon: "M8.228 9c.549-1.165 1.918-2 3.522-2 2.485 0 4.5 1.79 4.5 4 0 1.53-.967 2.86-2.39 3.53-.52.245-.86.75-.86 1.325V16m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" },
    { key: "general", labelKey: "settings.categoryGeneral", icon: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" },
    { key: "shortcuts", labelKey: "settings.categoryShortcuts", icon: "M13 10V3L4 14h7v7l9-11h-7z" },
    { key: "ai", labelKey: "settings.categoryAI", icon: "M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" },
    { key: "data", labelKey: "settings.categoryData", icon: "M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" },
  ];

  return (
    <div class="flex flex-col h-full bg-transparent text-[var(--cb-text)]">
      {/* 主体：左侧 Tab + 右侧内容 */}
      <div class="flex flex-1 overflow-hidden">
        {/* 左侧分类导航 */}
        <nav class="w-[160px] shrink-0 border-r border-[var(--cb-border)] flex flex-col py-2 px-2 gap-0.5">
          <button
            class="p-1.5 rounded-lg text-[var(--cb-text-3)] hover:text-[var(--cb-text)] hover:bg-[var(--cb-bg-hover)] transition-all self-start mb-2"
            onClick={props.onBack}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
            </svg>
          </button>
          {tabItems.map((tab) => (
            <button
              class={`flex items-center gap-2.5 px-3 py-2 rounded-xl text-[13px] font-medium transition-all text-left ${
                activeTab() === tab.key
                  ? "bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)] border border-[var(--cb-blue-text)]/30"
                  : "text-[var(--cb-text-3)] hover:bg-[var(--cb-bg-hover)] hover:text-[var(--cb-text-2)] border border-transparent"
              }`}
              onClick={() => setActiveTab(tab.key)}
            >
              <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d={tab.icon} />
              </svg>
              {t(tab.labelKey)}
            </button>
          ))}
          <div class="flex-1" />
          <p class="text-[10px] text-[var(--cb-text-4)] text-center px-2 pb-1">{t("app.name")} {t("app.version")}</p>
        </nav>

        {/* 右侧内容区 */}
        <div class="flex-1 overflow-y-auto p-5 space-y-5">

          {/* ═══════ 使用说明 ═══════ */}
          <Show when={activeTab() === "guide"}>
            <div class="space-y-5">
              <section class="space-y-2">
                <h3 class="text-[14px] font-semibold text-[var(--cb-text)]">{t("settings.guideTitle")}</h3>
                <p class="text-[12px] leading-5 text-[var(--cb-text-4)]">{t("settings.guideIntro")}</p>
              </section>

              <div class="grid gap-3">
                <For each={helpSections}>
                  {(section) => (
                    <section class="rounded-2xl border border-[var(--cb-border)] bg-[var(--cb-bg-card)] p-4">
                      <div class="space-y-1.5">
                        <h4 class="text-[13px] font-medium text-[var(--cb-text-2)]">{t(section.titleKey)}</h4>
                        <p class="text-[12px] leading-5 text-[var(--cb-text-4)]">{t(section.descKey)}</p>
                      </div>
                      <div class="mt-3 space-y-2">
                        <For each={section.bullets}>
                          {(bulletKey) => (
                            <div class="flex items-start gap-2 text-[12px] leading-5 text-[var(--cb-text-3)]">
                              <span class="mt-[5px] h-1.5 w-1.5 shrink-0 rounded-full bg-[var(--cb-blue-text)]/70" />
                              <span>{t(bulletKey)}</span>
                            </div>
                          )}
                        </For>
                      </div>
                    </section>
                  )}
                </For>
              </div>
            </div>
          </Show>

          {/* ═══════ 通用 ═══════ */}
          <Show when={activeTab() === "general"}>
            <div class="space-y-5">
              {/* 外观 */}
              <section class="space-y-2">
                <h3 class="text-[12px] font-medium text-[var(--cb-text-3)]">{t("settings.appearance")}</h3>
                <div class="flex gap-2">
                  {themeOptions.map((opt) => (
                    <button
                      class={`flex-1 px-3 py-2 rounded-xl text-[13px] font-medium transition-all border ${
                        theme() === opt.value
                          ? "bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)] border-[var(--cb-blue-text)]/30"
                          : "bg-[var(--cb-bg-card)] text-[var(--cb-text-2)] border-[var(--cb-border)] hover:bg-[var(--cb-bg-hover)]"
                      }`}
                      onClick={() => setTheme(opt.value)}
                    >
                      {t(opt.labelKey)}
                    </button>
                  ))}
                </div>
              </section>

              {/* 语言 */}
              <section class="space-y-2">
                <h3 class="text-[12px] font-medium text-[var(--cb-text-3)]">{t("settings.language")}</h3>
                <div class="flex gap-2">
                  {localeOptions.map((opt) => (
                    <button
                      class={`flex-1 px-3 py-2 rounded-xl text-[13px] font-medium transition-all border ${
                        locale() === opt.value
                          ? "bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)] border-[var(--cb-blue-text)]/30"
                          : "bg-[var(--cb-bg-card)] text-[var(--cb-text-2)] border-[var(--cb-border)] hover:bg-[var(--cb-bg-hover)]"
                      }`}
                      onClick={() => setLocale(opt.value)}
                    >
                      {opt.label}
                    </button>
                  ))}
                </div>
              </section>

              {/* 字体大小 */}
              <section class="space-y-2">
                <h3 class="text-[12px] font-medium text-[var(--cb-text-3)]">{t("settings.fontSize")}</h3>
                <div class="flex items-center gap-3">
                  <span class="text-[12px] text-[var(--cb-text-4)] shrink-0">A</span>
                  <input
                    type="range"
                    min={FONT_SIZE_MIN}
                    max={FONT_SIZE_MAX}
                    step={1}
                    value={fontSizePx()}
                    onInput={(e) => applyFontSize(Number(e.currentTarget.value))}
                    class="flex-1 h-1.5 rounded-full appearance-none bg-[var(--cb-border)] accent-[var(--cb-blue-text)] cursor-pointer [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:bg-[var(--cb-blue-text)] [&::-webkit-slider-thumb]:shadow"
                  />
                  <span class="text-[16px] text-[var(--cb-text-4)] shrink-0">A</span>
                  <span class="text-[12px] text-[var(--cb-text-3)] ml-1 min-w-[36px] text-center">{fontSizePx()}px</span>
                </div>
              </section>

              <section class="space-y-2">
                <h3 class="text-[12px] font-medium text-[var(--cb-text-3)]">{t("settings.startup")}</h3>
                <button
                  class={`w-full flex items-center justify-between gap-3 px-4 py-3 rounded-xl border transition-all ${
                    autoStart()
                      ? "bg-[var(--cb-blue-bg)] border-[var(--cb-blue-text)]/30"
                      : "bg-[var(--cb-bg-card)] border-[var(--cb-border)] hover:bg-[var(--cb-bg-hover)]"
                  }`}
                  onClick={() => void handleToggleAutoStart(!autoStart())}
                >
                  <div class="text-left">
                    <div class="text-[13px] font-medium text-[var(--cb-text-2)]">{t("settings.autoStart")}</div>
                    <p class="text-[11px] text-[var(--cb-text-4)] mt-0.5">{t("settings.autoStartDesc")}</p>
                  </div>
                  <span
                    class={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                      autoStart() ? "bg-[var(--cb-blue-text)]" : "bg-[var(--cb-border-strong)]"
                    }`}
                    aria-hidden="true"
                  >
                    <span
                      class={`inline-block h-5 w-5 transform rounded-full bg-white transition-transform ${
                        autoStart() ? "translate-x-5" : "translate-x-0.5"
                      }`}
                    />
                  </span>
                </button>
                <button
                  class={`w-full flex items-center justify-between gap-3 px-4 py-3 rounded-xl border transition-all ${
                    clearInputsOnPanelOpen()
                      ? "bg-[var(--cb-blue-bg)] border-[var(--cb-blue-text)]/30"
                      : "bg-[var(--cb-bg-card)] border-[var(--cb-border)] hover:bg-[var(--cb-bg-hover)]"
                  }`}
                  onClick={() => void handleToggleClearInputsOnPanelOpen(!clearInputsOnPanelOpen())}
                >
                  <div class="text-left">
                    <div class="text-[13px] font-medium text-[var(--cb-text-2)]">{t("settings.clearInputsOnPanelOpen")}</div>
                    <p class="text-[11px] text-[var(--cb-text-4)] mt-0.5">{t("settings.clearInputsOnPanelOpenDesc")}</p>
                  </div>
                  <span
                    class={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                      clearInputsOnPanelOpen() ? "bg-[var(--cb-blue-text)]" : "bg-[var(--cb-border-strong)]"
                    }`}
                    aria-hidden="true"
                  >
                    <span
                      class={`inline-block h-5 w-5 transform rounded-full bg-white transition-transform ${
                        clearInputsOnPanelOpen() ? "translate-x-5" : "translate-x-0.5"
                      }`}
                    />
                  </span>
                </button>
              </section>

              <section class="space-y-2">
                <h3 class="text-[12px] font-medium text-[var(--cb-text-3)]">{t("settings.panelLayout")}</h3>
                <button
                  class={`w-full flex items-center justify-between gap-3 px-4 py-3 rounded-xl border transition-all ${
                    showDetailPanelByDefault()
                      ? "bg-[var(--cb-blue-bg)] border-[var(--cb-blue-text)]/30"
                      : "bg-[var(--cb-bg-card)] border-[var(--cb-border)] hover:bg-[var(--cb-bg-hover)]"
                  }`}
                  onClick={() => void handleToggleDetailPanelByDefault(!showDetailPanelByDefault())}
                >
                  <div class="text-left">
                    <div class="text-[13px] font-medium text-[var(--cb-text-2)]">{t("settings.showDetailPanelByDefault")}</div>
                    <p class="text-[11px] text-[var(--cb-text-4)] mt-0.5">{t("settings.showDetailPanelByDefaultDesc")}</p>
                  </div>
                  <span
                    class={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                      showDetailPanelByDefault() ? "bg-[var(--cb-blue-text)]" : "bg-[var(--cb-border-strong)]"
                    }`}
                    aria-hidden="true"
                  >
                    <span
                      class={`inline-block h-5 w-5 transform rounded-full bg-white transition-transform ${
                        showDetailPanelByDefault() ? "translate-x-5" : "translate-x-0.5"
                      }`}
                    />
                  </span>
                </button>
              </section>

              <section class="space-y-2">
                <h3 class="text-[12px] font-medium text-[var(--cb-text-3)]">{t("settings.searchToolbar")}</h3>
                <button
                  class={`w-full flex items-center justify-between gap-3 px-4 py-3 rounded-xl border transition-all ${
                    showSearchToolbarButtons()
                      ? "bg-[var(--cb-blue-bg)] border-[var(--cb-blue-text)]/30"
                      : "bg-[var(--cb-bg-card)] border-[var(--cb-border)] hover:bg-[var(--cb-bg-hover)]"
                  }`}
                  onClick={() => void handleToggleSearchToolbarButtons(!showSearchToolbarButtons())}
                >
                  <div class="text-left">
                    <div class="text-[13px] font-medium text-[var(--cb-text-2)]">{t("settings.searchToolbarShowActions")}</div>
                    <p class="text-[11px] text-[var(--cb-text-4)] mt-0.5">{t("settings.searchToolbarDesc")}</p>
                  </div>
                  <span
                    class={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                      showSearchToolbarButtons() ? "bg-[var(--cb-blue-text)]" : "bg-[var(--cb-border-strong)]"
                    }`}
                    aria-hidden="true"
                  >
                    <span
                      class={`inline-block h-5 w-5 transform rounded-full bg-white transition-transform ${
                        showSearchToolbarButtons() ? "translate-x-5" : "translate-x-0.5"
                      }`}
                    />
                  </span>
                </button>
              </section>
            </div>
          </Show>

          {/* ═══════ 快捷键 ═══════ */}
          <Show when={activeTab() === "shortcuts"}>
            <div class="space-y-5">
              {/* 唤起面板快捷键 */}
              <section class="space-y-2">
                <h3 class="text-[12px] font-medium text-[var(--cb-text-3)]">{t("settings.hotkey")}</h3>
                <div class="flex items-center gap-3">
                  <span class="text-[13px] text-[var(--cb-text-2)] flex-shrink-0">{t("settings.openPanel")}</span>
                  <button
                    class={`flex-1 px-3 py-2.5 rounded-xl text-[13px] font-medium text-center transition-all border ${
                      recording()
                        ? "bg-[var(--cb-blue-bg)] border-[var(--cb-blue-text)]/40 text-[var(--cb-blue-text)] animate-pulse"
                        : "bg-[var(--cb-bg-card)] border-[var(--cb-border)] text-[var(--cb-text)] hover:bg-[var(--cb-bg-hover)]"
                    }`}
                    onClick={() => { setRecording(true); setShortcutMsg(null); }}
                    onKeyDown={(e) => { e.preventDefault(); }}
                  >
                    {recording() ? t("settings.recordingHint") : formatShortcut(currentShortcut())}
                  </button>
                </div>
                <Show when={shortcutMsg()}>
                  {(msg) => (
                    <div
                      class={`p-2 rounded-xl text-[12px] ${
                        msg().type === "success"
                          ? "bg-[var(--cb-emerald-bg)] text-[var(--cb-emerald-text)]"
                          : "bg-[var(--cb-red-bg)] text-[var(--cb-red-text)]"
                      }`}
                    >
                      {msg().text}
                    </div>
                  )}
                </Show>
              </section>

              {/* 内置快捷键说明 */}
              <section class="space-y-2">
                <h3 class="text-[12px] font-medium text-[var(--cb-text-3)]">{t("settings.builtinShortcuts")}</h3>
                <div class="px-4 py-3 rounded-xl border border-[var(--cb-border)] bg-[var(--cb-bg-card)] flex items-center justify-between gap-3">
                  <div class="text-left">
                    <div class="text-[13px] font-medium text-[var(--cb-text-2)]">{t("settings.toggleDetailPanelShortcut")}</div>
                    <p class="text-[11px] text-[var(--cb-text-4)] mt-0.5">{t("settings.toggleDetailPanelShortcutDesc")}</p>
                  </div>
                  <kbd class="px-2 py-1 rounded-md text-[11px] font-medium bg-[var(--cb-bg-hover)] border border-[var(--cb-border)] text-[var(--cb-text-2)] flex-shrink-0">Shift + Tab</kbd>
                </div>
              </section>

            </div>
          </Show>

          {/* ═══════ AI 模型 ═══════ */}
          <Show when={activeTab() === "ai"}>
            <div class="space-y-5">
              {/* 已保存的模型配置列表 */}
              <section class="space-y-3">
                <div class="flex items-center justify-between">
                  <div>
                    <h3 class="text-[12px] font-medium text-[var(--cb-text-3)]">{t("settings.savedConfigs")}</h3>
                    <p class="text-[11px] text-[var(--cb-text-4)] mt-0.5">{t("settings.aiModelDesc")}</p>
                  </div>
                  <button
                    class="px-2.5 py-1 rounded-lg text-[12px] font-medium bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)] hover:opacity-80 transition-all"
                    onClick={handleNewConfig}
                  >
                    + {t("settings.addConfig")}
                  </button>
                </div>

                <Show when={savedConfigs().length === 0 && !editingConfig()}>
                  <p class="text-[12px] text-[var(--cb-text-4)] text-center py-4">{t("settings.noConfigs")}</p>
                </Show>

                <For each={savedConfigs()}>
                  {(cfg) => (
                    <div class={`flex items-center gap-2.5 p-3 rounded-xl border transition-all ${
                      cfg.is_active
                        ? "bg-[var(--cb-blue-bg)] border-[var(--cb-blue-text)]/20"
                        : "bg-[var(--cb-bg-card)] border-[var(--cb-border)]"
                    }`}>
                      <button
                        class={`w-4 h-4 rounded-full border-2 shrink-0 transition-all ${
                          cfg.is_active
                            ? "border-[var(--cb-blue-text)] bg-[var(--cb-blue-text)]"
                            : "border-[var(--cb-text-4)] hover:border-[var(--cb-blue-text)]"
                        }`}
                        onClick={() => handleSetActive(cfg.name)}
                        title={t("settings.switchTo")}
                      >
                        <Show when={cfg.is_active}>
                          <div class="w-1.5 h-1.5 bg-white rounded-full mx-auto" />
                        </Show>
                      </button>
                      <div class="flex-1 min-w-0">
                        <div class="flex items-center gap-2">
                          <span class={`text-[13px] font-medium truncate ${
                            cfg.is_active ? "text-[var(--cb-blue-text)]" : "text-[var(--cb-text)]"
                          }`}>{cfg.name}</span>
                          <Show when={cfg.is_active}>
                            <span class="text-[10px] px-1.5 py-0.5 rounded-md bg-[var(--cb-blue-text)]/15 text-[var(--cb-blue-text)] font-medium">{t("settings.active")}</span>
                          </Show>
                        </div>
                        <p class="text-[11px] text-[var(--cb-text-4)] truncate">{cfg.model} · {cfg.base_url}</p>
                      </div>
                      <button
                        class="p-1 text-[var(--cb-text-4)] hover:text-[var(--cb-text)] transition-colors"
                        onClick={() => handleEditConfig(cfg)}
                        title={t("settings.editConfig")}
                      >
                        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                        </svg>
                      </button>
                      <button
                        class="p-1 text-[var(--cb-text-4)] hover:text-[var(--cb-red-text)] transition-colors"
                        onClick={() => handleDeleteConfig(cfg.name)}
                        title={t("common.delete")}
                      >
                        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                        </svg>
                      </button>
                    </div>
                  )}
                </For>

                <Show when={message()}>
                  {(msg) => (
                    <div
                      class={`p-2.5 rounded-xl text-[12px] ${
                        msg().type === "success"
                          ? "bg-[var(--cb-emerald-bg)] text-[var(--cb-emerald-text)]"
                          : "bg-[var(--cb-red-bg)] text-[var(--cb-red-text)]"
                      }`}
                    >
                      {msg().text}
                    </div>
                  )}
                </Show>
              </section>

              {/* 编辑/新增模型配置表单 */}
              <Show when={editingConfig()}>
                <section class="space-y-3 p-3 bg-[var(--cb-bg-card)] border border-[var(--cb-border)] rounded-xl">
                  <div class="flex items-center justify-between">
                    <h3 class="text-[12px] font-medium text-[var(--cb-text-3)]">
                      {name() ? t("settings.editConfig") : t("settings.addConfig")}
                    </h3>
                    <button
                      class="p-1 text-[var(--cb-text-4)] hover:text-[var(--cb-text)] transition-colors"
                      onClick={() => setEditingConfig(false)}
                    >
                      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                      </svg>
                    </button>
                  </div>

                  <div class="space-y-3">
                    <Field label={t("settings.configName")} value={name()} onInput={setName} placeholder="default" />
                    <Field label={t("settings.baseUrl")} value={baseUrl()} onInput={setBaseUrl} placeholder="https://api.openai.com/v1" />
                    <Field
                      label={t("settings.apiKey")}
                      value={apiKey()}
                      onInput={setApiKey}
                      placeholder="sk-..."
                      type={showApiKey() ? "text" : "password"}
                      trailingButton={
                        <button
                          type="button"
                          class="inline-flex h-8 w-8 items-center justify-center rounded-lg text-[var(--cb-text-4)] transition-all hover:bg-[var(--cb-bg-hover)] hover:text-[var(--cb-text-2)]"
                          onClick={() => setShowApiKey((value) => !value)}
                          title={showApiKey() ? t("settings.hideSecret") : t("settings.showSecret")}
                        >
                          <Show
                            when={showApiKey()}
                            fallback={
                              <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                              </svg>
                            }
                          >
                            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M3 3l18 18" />
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M10.585 10.587A2 2 0 0013.414 13.414" />
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M9.88 5.09A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.542 7a9.97 9.97 0 01-4.132 5.104M6.228 6.228A9.965 9.965 0 002.458 12c1.274 4.057 5.064 7 9.542 7 1.664 0 3.234-.407 4.614-1.127" />
                            </svg>
                          </Show>
                        </button>
                      }
                    />
                    <Field label={t("settings.modelName")} value={model()} onInput={setModel} placeholder="gpt-4o" />

                    <div class="grid grid-cols-2 gap-3">
                      <div>
                        <label class="block text-[12px] text-[var(--cb-text-3)] mb-1">{t("settings.timeout")}</label>
                        <input
                          type="number"
                          value={timeoutSecs()}
                          onInput={(e) => setTimeoutSecs(Number(e.currentTarget.value))}
                          class="w-full px-3 py-2 bg-[var(--cb-bg-input)] border border-[var(--cb-border)] rounded-xl text-[14px] text-[var(--cb-text-2)] focus:border-[var(--cb-blue-text)] focus:outline-none transition-all"
                        />
                      </div>
                      <div>
                        <label class="block text-[12px] text-[var(--cb-text-3)] mb-1">{t("settings.maxTokens")}</label>
                        <input
                          type="number"
                          value={maxTokens()}
                          onInput={(e) => setMaxTokens(Number(e.currentTarget.value))}
                          class="w-full px-3 py-2 bg-[var(--cb-bg-input)] border border-[var(--cb-border)] rounded-xl text-[14px] text-[var(--cb-text-2)] focus:border-[var(--cb-blue-text)] focus:outline-none transition-all"
                        />
                      </div>
                    </div>
                  </div>

                  {/* 快速填充 */}
                  <div class="flex gap-1.5">
                    <button class="px-2 py-1 rounded-lg text-[11px] bg-[var(--cb-bg-hover)] text-[var(--cb-text-3)] hover:text-[var(--cb-text)] transition-all"
                      onClick={() => { setBaseUrl("https://api.deepseek.com/v1"); setModel("deepseek-chat"); if (!name()) setName("deepseek"); }}
                    >DeepSeek</button>
                    <button class="px-2 py-1 rounded-lg text-[11px] bg-[var(--cb-bg-hover)] text-[var(--cb-text-3)] hover:text-[var(--cb-text)] transition-all"
                      onClick={() => { setBaseUrl("http://localhost:11434/v1"); setModel("qwen2.5:7b"); setApiKey("ollama"); if (!name()) setName("ollama"); }}
                    >{t("settings.ollamaLocal")}</button>
                    <button class="px-2 py-1 rounded-lg text-[11px] bg-[var(--cb-bg-hover)] text-[var(--cb-text-3)] hover:text-[var(--cb-text)] transition-all"
                      onClick={() => { setBaseUrl("https://api.openai.com/v1"); setModel("gpt-4o"); if (!name()) setName("openai"); }}
                    >OpenAI</button>
                  </div>

                  <button
                    class="w-full px-4 py-2.5 bg-[var(--cb-blue-bg)] hover:opacity-80 text-[var(--cb-blue-text)] disabled:opacity-40 rounded-xl text-[13px] font-medium transition-all flex items-center justify-center gap-2"
                    onClick={handleSave}
                    disabled={saving()}
                  >
                    <Show when={saving()}>
                      <div class="w-3.5 h-3.5 border-2 border-[var(--cb-blue-text)] border-t-transparent rounded-full animate-spin" />
                    </Show>
                    {saving() ? t("settings.savingAndTesting") : t("settings.saveAndTest")}
                  </button>
                </section>
              </Show>
            </div>
          </Show>

          {/* ═══════ 数据管理 ═══════ */}
          <Show when={activeTab() === "data"}>
            <div class="space-y-5">
              <section class="space-y-3">
                <p class="text-[12px] text-[var(--cb-text-4)]">{t("settings.dataManagementDesc")}</p>

                <div class="flex flex-wrap gap-1.5">
                  {retentionOptions.map((opt) => (
                    <button
                      class={`px-2.5 py-1.5 rounded-xl text-[12px] font-medium transition-all border ${
                        selectedRetention() === opt.days
                          ? "bg-[var(--cb-red-bg)] text-[var(--cb-red-text)] border-[var(--cb-red-text)]/30"
                          : "bg-[var(--cb-bg-card)] text-[var(--cb-text-3)] border-[var(--cb-border)] hover:bg-[var(--cb-bg-hover)]"
                      }`}
                      onClick={() => { setSelectedRetention(opt.days); setConfirmStep(false); setClearMsg(null); }}
                    >
                      {t(opt.labelKey)}
                    </button>
                  ))}
                </div>

                <Show when={!confirmStep()}>
                  <button
                    class="w-full px-4 py-2.5 bg-[var(--cb-bg-card)] hover:bg-[var(--cb-bg-hover)] border border-[var(--cb-border)] rounded-xl text-[13px] font-medium text-[var(--cb-red-text)] transition-all"
                    onClick={() => setConfirmStep(true)}
                  >
                    {selectedRetention() === 0
                      ? t("settings.clearAll")
                      : t("settings.clearBefore", { period: t(retentionOptions.find(o => o.days === selectedRetention())?.labelKey ?? "") })}
                  </button>
                </Show>
                <Show when={confirmStep()}>
                  <div class="flex gap-2">
                    <button
                      class="flex-1 px-4 py-2.5 bg-[var(--cb-red-bg)] border border-[var(--cb-red-text)]/20 rounded-xl text-[13px] font-medium text-[var(--cb-red-text)] transition-all hover:opacity-80 disabled:opacity-40 flex items-center justify-center gap-2"
                      onClick={handleClearData}
                      disabled={clearing()}
                    >
                      <Show when={clearing()}>
                        <div class="w-3.5 h-3.5 border-2 border-[var(--cb-red-text)] border-t-transparent rounded-full animate-spin" />
                      </Show>
                      {clearing() ? t("settings.clearing") : t("settings.confirmClear")}
                    </button>
                    <button
                      class="flex-1 px-4 py-2.5 bg-[var(--cb-bg-card)] border border-[var(--cb-border)] rounded-xl text-[13px] font-medium text-[var(--cb-text-2)] transition-all hover:bg-[var(--cb-bg-hover)]"
                      onClick={() => setConfirmStep(false)}
                    >
                      {t("common.cancel")}
                    </button>
                  </div>
                </Show>

                <Show when={clearMsg()}>
                  {(msg) => (
                    <div
                      class={`p-2 rounded-xl text-[12px] ${
                        msg().type === "success"
                          ? "bg-[var(--cb-emerald-bg)] text-[var(--cb-emerald-text)]"
                          : "bg-[var(--cb-red-bg)] text-[var(--cb-red-text)]"
                      }`}
                    >
                      {msg().text}
                    </div>
                  )}
                </Show>
              </section>

              {/* 按大小清理 */}
              <section class="space-y-3 pt-4 border-t border-[var(--cb-border)]">
                <div>
                  <h3 class="text-[13px] font-medium text-[var(--cb-text-2)]">{t("settings.clearBySize")}</h3>
                  <p class="text-[11px] text-[var(--cb-text-4)] mt-0.5">{t("settings.clearBySizeDesc")}</p>
                </div>

                <div class="flex items-center gap-2">
                  <input
                    type="number"
                    min={1}
                    step={1}
                    value={sizeThreshold()}
                    onInput={(e) => {
                      setSizeThreshold(Number(e.currentTarget.value));
                      setSizePreview(null);
                      setSizeConfirmStep(false);
                      setSizeMsg(null);
                    }}
                    class="flex-1 px-3 py-2 bg-[var(--cb-bg-input)] border border-[var(--cb-border)] rounded-xl text-[13px] text-[var(--cb-text-2)] focus:border-[var(--cb-blue-text)] focus:outline-none transition-all"
                  />
                  <div class="flex rounded-xl border border-[var(--cb-border)] overflow-hidden">
                    {(["KB", "MB"] as SizeUnit[]).map((u) => (
                      <button
                        class={`px-3 py-2 text-[12px] font-medium transition-all ${
                          sizeUnit() === u
                            ? "bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)]"
                            : "bg-[var(--cb-bg-card)] text-[var(--cb-text-3)] hover:bg-[var(--cb-bg-hover)]"
                        }`}
                        onClick={() => {
                          setSizeUnit(u);
                          setSizePreview(null);
                          setSizeConfirmStep(false);
                          setSizeMsg(null);
                        }}
                      >
                        {u}
                      </button>
                    ))}
                  </div>
                  <button
                    class="px-3 py-2 rounded-xl text-[12px] font-medium bg-[var(--cb-bg-card)] border border-[var(--cb-border)] text-[var(--cb-text-2)] hover:bg-[var(--cb-bg-hover)] transition-all disabled:opacity-40 flex items-center gap-1.5"
                    onClick={handleScanOverSize}
                    disabled={sizeScanning()}
                  >
                    <Show when={sizeScanning()}>
                      <div class="w-3 h-3 border-2 border-[var(--cb-text-3)] border-t-transparent rounded-full animate-spin" />
                    </Show>
                    {t("settings.scanOverSize")}
                  </button>
                </div>

                <p class="text-[11px] text-[var(--cb-text-4)]">
                  {t("settings.sizeThresholdHint", { bytes: formatBytes(thresholdBytes()) })}
                </p>

                <Show when={sizePreview()}>
                  {(preview) => (
                    <div class="p-3 rounded-xl bg-[var(--cb-bg-card)] border border-[var(--cb-border)] space-y-2">
                      <Show
                        when={preview().count > 0}
                        fallback={
                          <p class="text-[12px] text-[var(--cb-text-3)]">{t("settings.scanEmpty")}</p>
                        }
                      >
                        <p class="text-[12px] text-[var(--cb-text-2)]">
                          {t("settings.scanResult", {
                            count: preview().count,
                            size: formatBytes(preview().totalBytes),
                          })}
                        </p>
                        <Show when={!sizeConfirmStep()}>
                          <button
                            class="w-full px-4 py-2 bg-[var(--cb-red-bg)] border border-[var(--cb-red-text)]/20 rounded-xl text-[12px] font-medium text-[var(--cb-red-text)] hover:opacity-80 transition-all"
                            onClick={() => setSizeConfirmStep(true)}
                          >
                            {t("settings.clearBySizeAction", { count: preview().count })}
                          </button>
                        </Show>
                        <Show when={sizeConfirmStep()}>
                          <div class="flex gap-2">
                            <button
                              class="flex-1 px-4 py-2 bg-[var(--cb-red-bg)] border border-[var(--cb-red-text)]/20 rounded-xl text-[12px] font-medium text-[var(--cb-red-text)] hover:opacity-80 transition-all disabled:opacity-40 flex items-center justify-center gap-2"
                              onClick={handleClearOverSize}
                              disabled={sizeClearing()}
                            >
                              <Show when={sizeClearing()}>
                                <div class="w-3 h-3 border-2 border-[var(--cb-red-text)] border-t-transparent rounded-full animate-spin" />
                              </Show>
                              {sizeClearing() ? t("settings.clearing") : t("settings.confirmClear")}
                            </button>
                            <button
                              class="flex-1 px-4 py-2 bg-[var(--cb-bg-card)] border border-[var(--cb-border)] rounded-xl text-[12px] font-medium text-[var(--cb-text-2)] hover:bg-[var(--cb-bg-hover)] transition-all"
                              onClick={() => setSizeConfirmStep(false)}
                            >
                              {t("common.cancel")}
                            </button>
                          </div>
                        </Show>
                      </Show>
                    </div>
                  )}
                </Show>

                <Show when={sizeMsg()}>
                  {(msg) => (
                    <div
                      class={`p-2 rounded-xl text-[12px] ${
                        msg().type === "success"
                          ? "bg-[var(--cb-emerald-bg)] text-[var(--cb-emerald-text)]"
                          : "bg-[var(--cb-red-bg)] text-[var(--cb-red-text)]"
                      }`}
                    >
                      {msg().text}
                    </div>
                  )}
                </Show>
              </section>
            </div>
          </Show>

        </div>
      </div>
    </div>
  );
};

// --- 子组件 ---

const Field: Component<{
  label: string;
  value: string;
  onInput: (v: string) => void;
  placeholder?: string;
  type?: string;
  trailingButton?: JSX.Element;
}> = (props) => (
  <div>
    <label class="block text-[12px] text-[var(--cb-text-3)] mb-1">{props.label}</label>
    <div class="relative">
      <input
        type={props.type ?? "text"}
        value={props.value}
        onInput={(e) => props.onInput(e.currentTarget.value)}
        placeholder={props.placeholder}
        class={`w-full bg-[var(--cb-bg-input)] border border-[var(--cb-border)] rounded-xl text-[14px] text-[var(--cb-text-2)] placeholder-[var(--cb-text-4)] focus:border-[var(--cb-blue-text)] focus:outline-none transition-all ${
          props.trailingButton ? "px-3 py-2 pr-11" : "px-3 py-2"
        }`}
      />
      <Show when={props.trailingButton}>
        <div class="absolute inset-y-0 right-2 flex items-center">
          {props.trailingButton}
        </div>
      </Show>
    </div>
  </div>
);

const QuickConfig: Component<{ label: string; onClick: () => void }> = (props) => (
  <button
    class="w-full px-3 py-2.5 bg-[var(--cb-bg-card)] hover:bg-[var(--cb-bg-hover)] border border-[var(--cb-border)] rounded-xl text-[14px] text-[var(--cb-text-2)] text-left transition-all"
    onClick={props.onClick}
  >
    {props.label}
  </button>
);

export default SettingsPage;
