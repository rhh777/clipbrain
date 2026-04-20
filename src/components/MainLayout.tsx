import { Component, createSignal, createEffect, createMemo, onMount, onCleanup, For, Show } from "solid-js";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import {
  getConfig,
  searchHistoryAdvanced,
  listHistory,
  deleteHistory,
  togglePin,
  writeHistoryItemToClipboard,
  restorePreviousAppAndPaste,
  writeToClipboard,
  listActions,
  executeAction,
  executeActionStream,
  executeCustomStream,
  stopActionStream,
  listAllTags,
  getTags,
  addTag,
  removeTag,
  type ClipboardHistoryItem,
} from "../lib/ipc";
import type {
  ClipboardChangeEvent,
  ActionDescriptor,
  ActionOutput,
  ActionStreamPayload,
  ContentType,
} from "../types/clipboard";
import HistoryList from "./HistoryList";
import DetailPanel from "./DetailPanel";
import DateRangePicker from "./DateRangePicker";
import { theme, setTheme } from "../lib/theme";
import { t, locale } from "../lib/i18n";
import trayIconUrl from "../../src-tauri/icons/tray-icon@2x.png";

const selectionTagCache = new Map<number, string[]>();
const selectionActionCache = new Map<string, ActionDescriptor[]>();

const contentTypeKeys = [
  { value: undefined as string | undefined, key: "all" },
  { value: "Json", key: "Json" },
  { value: "Yaml", key: "Yaml" },
  { value: "Url", key: "Url" },
  { value: "Code", key: "Code" },
  { value: "MathExpression", key: "MathExpression" },
  { value: "TableData", key: "TableData" },
  { value: "Image", key: "Image" },
  { value: "FileList", key: "FileList" },
  { value: "PlainText", key: "PlainText" },
];

const defaultVisibleContentTypeKeys = new Set(["all", "PlainText", "Image", "FileList"]);

interface MainLayoutProps {
  onOpenSettings: () => void;
  onOpenStats: () => void;
  onOpenPlugins: () => void;
}

type SlashFilterOption =
  | { kind: "reset"; group: "reset"; label: string; description: string }
  | { kind: "tag"; group: "tag"; value: string; label: string; description: string }
  | { kind: "contentType"; group: "type"; value: ContentType; label: string; description: string }
  | { kind: "favorite"; group: "status"; label: string; description: string };

const MainLayout: Component<MainLayoutProps> = (props) => {
  // --- 历史列表状态 ---
  const [items, setItems] = createSignal<ClipboardHistoryItem[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [keyword, setKeyword] = createSignal("");
  const [searchInputValue, setSearchInputValue] = createSignal("");
  const [typeFilter, setTypeFilter] = createSignal<string | undefined>(undefined);
  const [tagFilter, setTagFilter] = createSignal<string | undefined>(undefined);
  const [favoriteOnly, setFavoriteOnly] = createSignal(false);
  const [datePreset, setDatePreset] = createSignal<string>("all");
  const [dateFrom, setDateFrom] = createSignal<string | undefined>(undefined);
  const [dateTo, setDateTo] = createSignal<string | undefined>(undefined);
  const [showDatePicker, setShowDatePicker] = createSignal(false);
  const [allTags, setAllTags] = createSignal<string[]>([]);
  const [availableContentTypes, setAvailableContentTypes] = createSignal<string[]>([]);
  const [tagDropdownOpen, setTagDropdownOpen] = createSignal(false);
  const [tagSearchValue, setTagSearchValue] = createSignal("");
  const [page, setPage] = createSignal(0);
  const [hasMore, setHasMore] = createSignal(true);
  const [showSearchToolbarButtons, setShowSearchToolbarButtons] = createSignal(false);
  const [showDetailPanelByDefault, setShowDetailPanelByDefault] = createSignal(true);
  const [clearInputsOnPanelOpen, setClearInputsOnPanelOpen] = createSignal(false);
  const [showItemMeta, setShowItemMeta] = createSignal(true);
  const [detailInputClearToken, setDetailInputClearToken] = createSignal(0);
  const PAGE_SIZE = 50;

  // --- 选中与详情状态 ---
  const [selectedId, setSelectedId] = createSignal<number | null>(null);
  const [selectedItem, setSelectedItem] = createSignal<ClipboardHistoryItem | null>(null);
  const [itemTags, setItemTags] = createSignal<string[]>([]);

  // --- 当前剪贴板状态 ---
  const [currentClip, setCurrentClip] = createSignal<ClipboardChangeEvent | null>(null);

  // --- 操作 / 结果状态 ---
  const [actions, setActions] = createSignal<ActionDescriptor[]>([]);
  const [executing, setExecuting] = createSignal<string | null>(null);
  const [result, setResult] = createSignal<ActionOutput | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [copied, setCopied] = createSignal(false);
  const [streaming, setStreaming] = createSignal(false);
  const [streamContent, setStreamContent] = createSignal("");
  const [streamThinking, setStreamThinking] = createSignal("");
  const [thinking, setThinking] = createSignal(false);

  // --- 焦点区域 ---
  const [focusArea, setFocusArea] = createSignal<"left" | "right">("left");
  const [selectedIndex, setSelectedIndex] = createSignal(0);
  let unlistenClip: UnlistenFn | undefined;
  let unlistenMainWindowShown: UnlistenFn | undefined;
  let focusSearchOnWindowFocus: (() => void) | undefined;
  let blurActiveElementOnWindowBlur: (() => void) | undefined;
  let selectionRequestId = 0;
  let searchInputSelectionArmed = false;
  const [isSearchInputFocused, setIsSearchInputFocused] = createSignal(false);
  let tagDropdownRef: HTMLDivElement | undefined;
  let tagDropdownInputRef: HTMLInputElement | undefined;

  const slashCommandQuery = createMemo(() => {
    const value = searchInputValue();
    if (!value.startsWith("/")) return null;
    return value.slice(1).trim().toLowerCase();
  });

  const slashFilterOptions = createMemo<SlashFilterOption[]>(() => {
    const query = slashCommandQuery();
    if (query === null) return [];

    const resetOptions =
      query === "" && (tagFilter() || typeFilter() || favoriteOnly())
        ? [
            {
              kind: "reset",
              group: "reset",
              label: t("contentType.all"),
              description: t("search.commandResetFilters"),
            } satisfies SlashFilterOption,
          ]
        : [];

    const favoriteLabel = t("history.pin");
    const favoriteOptions =
      !query || favoriteLabel.toLowerCase().includes(query)
        ? [
            {
              kind: "favorite",
              group: "status",
              label: favoriteLabel,
              description: t("search.commandApplyFavorite"),
            } satisfies SlashFilterOption,
          ]
        : [];

    const tagOptions = allTags()
      .filter((tag) => !query || tag.toLowerCase().includes(query))
      .map(
        (tag) =>
            ({
              kind: "tag",
              group: "tag",
              value: tag,
              label: `#${tag}`,
              description: t("search.commandApplyTag"),
          }) satisfies SlashFilterOption
      );

    const builtinContentTypes: ContentType[] = ["Image", "FileList"];
    const builtinOptions = builtinContentTypes
      .filter((contentType) => {
        if (!query) return true;
        return t(`contentType.${contentType}`).toLowerCase().includes(query);
      })
      .map(
        (contentType) =>
            ({
              kind: "contentType",
              group: "type",
              value: contentType,
              label: t(`contentType.${contentType}`),
              description: t("search.commandApplyType"),
          }) satisfies SlashFilterOption
      );

    return [...resetOptions, ...builtinOptions, ...tagOptions, ...favoriteOptions];
  });

  const [slashTagIndex, setSlashTagIndex] = createSignal(0);

  const slashGroupLabel = (group: SlashFilterOption["group"]) => {
    switch (group) {
      case "type":
        return t("search.filterType");
      case "tag":
        return t("search.filterTag");
      case "status":
        return t("search.filterStatus");
      default:
        return t("search.filterReset");
    }
  };

  const slashCommandActive = createMemo(() => isSearchInputFocused() && slashCommandQuery() !== null);
  const showSlashDropdown = createMemo(() => {
    if (!slashCommandActive()) return false;
    const query = slashCommandQuery();
    return query === "" || slashFilterOptions().length > 0;
  });
  const visibleContentTypeKeys = createMemo(() => {
    const availableKeys = new Set(availableContentTypes());
    const activeType = typeFilter();

    return contentTypeKeys.filter((ct) => {
      if (defaultVisibleContentTypeKeys.has(ct.key)) return true;
      if (ct.value && availableKeys.has(ct.value)) return true;
      return Boolean(activeType && ct.value === activeType);
    });
  });
  const filteredTagOptions = createMemo(() => {
    const query = tagSearchValue().trim().toLowerCase();
    return allTags().filter((tag) => !query || tag.toLowerCase().includes(query));
  });

  const focusSearchInput = () => {
    setFocusArea("left");
    searchInputSelectionArmed = false;
    const input = document.querySelector<HTMLInputElement>("[data-search-input]");
    input?.focus();
    input?.select();
  };

  const blurActiveElement = () => {
    const active = document.activeElement;
    if (active instanceof HTMLElement) {
      active.blur();
    }
  };

  const isEditableTarget = (target: EventTarget | null) =>
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    (target instanceof HTMLElement && target.isContentEditable);

  const isSearchInputTarget = (target: EventTarget | null) =>
    target instanceof HTMLElement && target.matches("[data-search-input]");

  const handleLeftListNavigation = (key: string, list: ClipboardHistoryItem[]) => {
    if (list.length === 0) return false;

    const PAGE_STEP = 10;
    let nextIndex: number | null = null;

    if (key === "ArrowDown") {
      nextIndex = Math.min(selectedIndex() + 1, list.length - 1);
    } else if (key === "ArrowUp") {
      nextIndex = Math.max(selectedIndex() - 1, 0);
    } else if (key === "PageDown") {
      nextIndex = Math.min(selectedIndex() + PAGE_STEP, list.length - 1);
    } else if (key === "PageUp") {
      nextIndex = Math.max(selectedIndex() - PAGE_STEP, 0);
    } else if (key === "Home") {
      nextIndex = 0;
    } else if (key === "End") {
      nextIndex = list.length - 1;
    }

    if (nextIndex === null) return false;
    setSelectedIndex(nextIndex);
    if (list[nextIndex]) {
      void selectItem(list[nextIndex]);
    }
    searchInputSelectionArmed = true;
    return true;
  };

  const getSearchInputConfirmItem = (list: ClipboardHistoryItem[]) => {
    if (list.length === 0) return null;
    const current = selectedItem();
    if (current) {
      const currentIndex = list.findIndex((item) => item.id === current.id);
      if (currentIndex >= 0) {
        return { item: current, index: currentIndex };
      }
    }
    return { item: list[0], index: 0 };
  };

  const focusCustomActionInput = () => {
    const input = document.querySelector<HTMLInputElement>("[data-custom-action-input]");
    if (!input) return false;
    setFocusArea("right");
    input.focus();
    return true;
  };

  const toggleDetailPanel = (nextVisible?: boolean) => {
    const targetVisible = nextVisible ?? !showDetailPanelByDefault();
    if (targetVisible && !selectedItem()) {
      return false;
    }
    setShowDetailPanelByDefault(targetVisible);
    if (targetVisible) {
      blurActiveElement();
      setFocusArea("right");
    } else {
      setFocusArea("left");
    }
    return true;
  };

  const hasActiveFilters = () =>
    Boolean(keyword().trim() || typeFilter() || tagFilter() || favoriteOnly() || dateFrom() || dateTo());

  const clearPanelInputs = () => {
    updateSearchInput("");
    setDetailInputClearToken((token) => token + 1);
  };

  const updateSearchInput = (value: string) => {
    setSearchInputValue(value);
    if (value.startsWith("/")) {
      setKeyword("");
      setSlashTagIndex(0);
      return;
    }
    setKeyword(value);
  };

  const applySlashFilter = (option: SlashFilterOption) => {
    if (option.kind === "reset") {
      setTagFilter(undefined);
      setTypeFilter(undefined);
      setFavoriteOnly(false);
    } else if (option.kind === "tag") {
      setTagFilter(option.value);
      setTypeFilter(undefined);
      setFavoriteOnly(false);
    } else if (option.kind === "favorite") {
      setTagFilter(undefined);
      setTypeFilter(undefined);
      setFavoriteOnly(true);
    } else {
      setTypeFilter(option.value);
      setTagFilter(undefined);
      setFavoriteOnly(false);
    }
    setSearchInputValue("");
    setKeyword("");
    setSlashTagIndex(0);
    searchInputSelectionArmed = false;
    const input = document.querySelector<HTMLInputElement>("[data-search-input]");
    input?.focus();
  };

  const closeTagDropdown = (resetSearch = true) => {
    setTagDropdownOpen(false);
    if (resetSearch) setTagSearchValue("");
  };

  const applyTagDropdownFilter = (tag?: string) => {
    setTagFilter(tag);
    setSearchInputValue("");
    setKeyword("");
    closeTagDropdown();
  };

  const handleTagDropdownPointerDown = (event: PointerEvent) => {
    const target = event.target;
    if (!(tagDropdownOpen() && tagDropdownRef && target instanceof Node)) return;
    if (!tagDropdownRef.contains(target)) {
      closeTagDropdown();
    }
  };

  const rememberAvailableContentTypes = (historyItems: ClipboardHistoryItem[], reset = false) => {
    const nextKeys = historyItems.map((item) => normalizeContentTypeKey(item.content_type));
    setAvailableContentTypes((prev) => {
      const merged = reset ? new Set<string>() : new Set(prev);
      for (const key of nextKeys) {
        merged.add(key);
      }
      return Array.from(merged);
    });
  };

  const mergeIncomingItem = (prev: ClipboardHistoryItem[], incoming: ClipboardHistoryItem): ClipboardHistoryItem[] => {
    const merged = [incoming, ...prev.filter((item) => item.id !== incoming.id)];
    return merged.slice(0, Math.max(prev.length, 1));
  };

  // --- 数据获取 ---
  const fetchItems = async (reset = false) => {
    setLoading(true);
    try {
      const offset = reset ? 0 : page() * PAGE_SIZE;
      const kw = keyword().trim() || undefined;
      const ct = typeFilter();
      const tg = tagFilter();
      const pinnedOnly = favoriteOnly() ? true : undefined;

      const df = dateFrom();
      const dt = dateTo();

      let result: ClipboardHistoryItem[];
      if (kw || ct || tg || pinnedOnly || df || dt) {
        result = await searchHistoryAdvanced(kw, ct, tg, pinnedOnly, df, dt, PAGE_SIZE, offset);
      } else {
        result = await listHistory(PAGE_SIZE, offset);
        rememberAvailableContentTypes(result, reset);
      }

      setHasMore(result.length === PAGE_SIZE);
      if (reset) {
        setItems(result);
        setPage(1);
        // 自动选中第一条
        if (result.length > 0) {
          setSelectedIndex(0);
          selectItem(result[0]);
        } else {
          setSelectedItem(null);
          setActions([]);
        }
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

  const refreshTags = async () => {
    try {
      const tags = await listAllTags();
      setAllTags(tags);
    } catch (e) {
      console.error("Failed to load tags:", e);
    }
  };

  const selectItem = async (item: ClipboardHistoryItem) => {
    if (selectedId() === item.id && selectedItem()?.id === item.id) {
      return;
    }

    setSelectedId(item.id);
    setSelectedItem(item);
    setResult(null);
    setError(null);
    setCopied(false);
    const requestId = ++selectionRequestId;
    const selectionStart = performance.now();

    const tagsPromise = (() => {
      const cached = selectionTagCache.get(item.id);
      if (cached) {
        return Promise.resolve(cached);
      }
      return getTags(item.id).then((tags) => {
        selectionTagCache.set(item.id, tags);
        return tags;
      });
    })().catch(() => [] as string[]);

    const actionsPromise = (() => {
      const ct = parseContentType(item.content_type);
      const actionInput = resolveActionInput(item);
      if (!actionInput) {
        return Promise.resolve([] as ActionDescriptor[]);
      }
      const key = `${locale()}::${item.content_type}`;
      const cached = selectionActionCache.get(key);
      if (cached) {
        return Promise.resolve(cached);
      }
      return listActions(ct, locale()).then((actionList) => {
        selectionActionCache.set(key, actionList);
        return actionList;
      });
    })().catch(() => [] as ActionDescriptor[]);

    const [tags, actionList] = await Promise.all([tagsPromise, actionsPromise]);
    if (requestId !== selectionRequestId) {
      return;
    }

    setItemTags(tags);
    setActions(actionList);

    const duration = performance.now() - selectionStart;
    if (duration > 80) {
      console.warn("[ClipBrain][perf] slow selectItem", {
        id: item.id,
        contentType: item.content_type,
        durationMs: Math.round(duration),
      });
    }
  };

  // --- 操作执行 ---
  const handleAction = async (action: ActionDescriptor) => {
    const item = selectedItem();
    if (!item) return;

    setExecuting(action.id);
    setResult(null);
    setError(null);
    setCopied(false);
    setStreamContent("");
    setStreamThinking("");

    const ct = parseContentType(item.content_type);
    const actionInput = resolveActionInput(item);
    if (!actionInput) {
      setError("No content available for this action");
      setExecuting(null);
      return;
    }

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
            case "done":
            case "error":
              setStreaming(false);
              setExecuting(null);
              break;
            case "cancelled":
              if (streamContent()) {
                setResult({ result: streamContent(), result_type: "text" });
              }
              setStreaming(false);
              setExecuting(null);
              break;
          }
        });

        const thinkingVal = thinking() ? undefined : false;
        const output = await executeActionStream(action.id, actionInput, ct, thinkingVal);
        setResult(output);
      } catch (e: any) {
        const message = typeof e === "string" ? e : e?.message ?? t("detail.actionFailed");
        if (message !== "操作已取消") {
          setError(message);
        }
      } finally {
        unlistenStream?.();
        setStreaming(false);
        setExecuting(null);
      }
    } else {
      // 非模型操作：直接执行
      try {
        const thinkingVal = thinking() ? undefined : false;
        const output = await executeAction(action.id, actionInput, ct, thinkingVal);
        setResult(output);
      } catch (e: any) {
        setError(typeof e === "string" ? e : e?.message ?? t("detail.actionFailed"));
      } finally {
        setExecuting(null);
      }
    }
  };

  // --- 自定义操作 ---
  const handleCustomAction = async (prompt: string) => {
    const item = selectedItem();
    if (!item) return;
    const contentType = parseContentType(item.content_type);
    const actionInput = resolveActionInput(item);
    if (!actionInput) return;

    setExecuting("custom_prompt");
    setResult(null);
    setError(null);
    setCopied(false);
    setStreamContent("");
    setStreamThinking("");
    setStreaming(true);

    let unlistenStream: UnlistenFn | undefined;
    try {
      unlistenStream = await listen<ActionStreamPayload>("action-stream", (event) => {
        const p = event.payload;
        if (p.action_id !== "custom_prompt") return;
        switch (p.event_type) {
          case "thinking":
            setStreamThinking((prev) => prev + p.content);
            break;
          case "delta":
            setStreamContent((prev) => prev + p.content);
            break;
          case "done":
          case "error":
            setStreaming(false);
            setExecuting(null);
            break;
          case "cancelled":
            if (streamContent()) {
              setResult({ result: streamContent(), result_type: "text" });
            }
            setStreaming(false);
            setExecuting(null);
            break;
        }
      });

      const thinkingVal = thinking() ? undefined : false;
      const output = await executeCustomStream(actionInput, contentType, prompt, thinkingVal);
      setResult(output);
    } catch (e: any) {
      const message = typeof e === "string" ? e : e?.message ?? t("detail.actionFailed");
      if (message !== "操作已取消") {
        setError(message);
      }
    } finally {
      unlistenStream?.();
      setStreaming(false);
      setExecuting(null);
    }
  };

  const handleCopyResult = async () => {
    const text = result()?.result || streamContent();
    if (!text) return;
    try {
      await writeToClipboard(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {}
  };

  const handleCopyItem = async (item: ClipboardHistoryItem) => {
    try {
      await writeHistoryItemToClipboard(item);
    } catch {}
  };

  const confirmHistorySelection = async (item: ClipboardHistoryItem) => {
    try {
      setError(null);
      await writeHistoryItemToClipboard(item);
      blurActiveElement();
      await Promise.all([
        invoke("hide_overlay_panel"),
        restorePreviousAppAndPaste(),
      ]);
    } catch (e: any) {
      const message = typeof e === "string" ? e : e?.message ?? t("clipboard.actionFailed");
      if (document.visibilityState === "visible") {
        setError(message);
      } else {
        console.error("Failed to confirm history selection:", e);
      }
    }
  };

  const handleStopStreaming = async () => {
    const actionId = executing();
    if (!actionId) return;
    try {
      await stopActionStream(actionId);
    } catch {}
    if (streamContent()) {
      setResult({ result: streamContent(), result_type: "text" });
    }
    setStreaming(false);
    setExecuting(null);
  };

  const handleTogglePin = async (id: number) => {
    try {
      const newState = await togglePin(id);
      setItems((prev) =>
        prev.map((i) => (i.id === id ? { ...i, is_pinned: newState } : i))
      );
      const sel = selectedItem();
      if (sel && sel.id === id) {
        setSelectedItem({ ...sel, is_pinned: newState });
      }
      if (favoriteOnly() && !newState) {
        void fetchItems(true);
      }
    } catch {}
  };

  const handleDelete = async (id: number) => {
    try {
      await deleteHistory(id);
      setItems((prev) => prev.filter((i) => i.id !== id));
      if (selectedId() === id) {
        setSelectedItem(null);
        setActions([]);
      }
    } catch {}
  };

  const handleAddTag = async (tagName: string) => {
    const item = selectedItem();
    if (!item) return;
    try {
      await addTag(item.id, tagName);
      setItemTags((prev) => {
        const next = [...prev, tagName];
        selectionTagCache.set(item.id, next);
        return next;
      });
      refreshTags();
    } catch {}
  };

  const handleRemoveTag = async (tagName: string) => {
    const item = selectedItem();
    if (!item) return;
    try {
      await removeTag(item.id, tagName);
      setItemTags((prev) => {
        const next = prev.filter((t) => t !== tagName);
        selectionTagCache.set(item.id, next);
        return next;
      });
      refreshTags();
    } catch {}
  };

  // --- 键盘导航 ---
  const handleKeyDown = (e: KeyboardEvent) => {
    const list = items();

    if (e.key === "Tab" && e.shiftKey) {
      if (toggleDetailPanel()) {
        e.preventDefault();
        e.stopPropagation();
      }
      return;
    }

    // Tab 切换焦点区域
    if (e.key === "Tab") {
      e.preventDefault();
      if (focusArea() === "left") {
        if (!focusCustomActionInput()) {
          focusSearchInput();
        }
      } else {
        focusSearchInput();
      }
      return;
    }

    // 搜索聚焦
    if ((e.metaKey && e.key === "f") || (e.key === "/" && focusArea() === "left" && !isEditableTarget(e.target))) {
      e.preventDefault();
      focusSearchInput();
      return;
    }

    // Escape
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      if (tagDropdownOpen()) {
        closeTagDropdown();
      } else if (showDatePicker()) {
        setShowDatePicker(false);
      } else if (searchInputValue()) {
        updateSearchInput("");
      } else if (tagFilter()) {
        setTagFilter(undefined);
      } else if (favoriteOnly()) {
        setFavoriteOnly(false);
      } else {
        blurActiveElement();
        invoke("hide_overlay_panel");
      }
      return;
    }

    if (focusArea() === "left" && isSearchInputTarget(e.target) && slashCommandActive()) {
      const options = slashFilterOptions();

      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        if (options.length === 0) return;
        const delta = e.key === "ArrowDown" ? 1 : -1;
        setSlashTagIndex((current) => {
          const next = current + delta;
          if (next < 0) return options.length - 1;
          if (next >= options.length) return 0;
          return next;
        });
        return;
      }

      if (e.key === "Enter" && !e.isComposing) {
        e.preventDefault();
        const option = options[slashTagIndex()] ?? options[0];
        if (option) {
          applySlashFilter(option);
        }
        return;
      }
    }

    if (focusArea() === "left" && isSearchInputTarget(e.target) && handleLeftListNavigation(e.key, list)) {
      e.preventDefault();
      return;
    }

    if (
      focusArea() === "left" &&
      isSearchInputTarget(e.target) &&
      e.key === "Enter" &&
      !e.isComposing &&
      !slashCommandActive()
    ) {
      e.preventDefault();
      const target = getSearchInputConfirmItem(list);
      searchInputSelectionArmed = false;
      if (target) {
        setSelectedIndex(target.index);
        void selectItem(target.item);
        void confirmHistorySelection(target.item);
      }
      return;
    }

    if (isEditableTarget(e.target)) {
      return;
    }

    if (focusArea() === "left") {
      if (handleLeftListNavigation(e.key, list)) {
        e.preventDefault();
      } else if (e.key === "Enter") {
        e.preventDefault();
        const item = selectedItem();
        if (item) {
          void confirmHistorySelection(item);
        }
      }
    }

    if (focusArea() === "right") {
      // ⌘1-9 快捷执行操作
      if (e.metaKey && e.key >= "1" && e.key <= "9") {
        e.preventDefault();
        const idx = parseInt(e.key) - 1;
        const act = actions();
        if (act[idx]) handleAction(act[idx]);
      }
      // ⌘C 复制结果
      if (e.metaKey && e.key === "c" && result()) {
        e.preventDefault();
        handleCopyResult();
      }
    }
  };

  // --- 剪贴板变化事件 ---
  onMount(async () => {
    try {
      const cfg = await getConfig();
      setShowDetailPanelByDefault(cfg.general.show_detail_panel_by_default ?? true);
      setShowSearchToolbarButtons(cfg.general.show_search_toolbar_buttons ?? false);
      setClearInputsOnPanelOpen(cfg.general.clear_inputs_on_panel_open ?? false);
      setShowItemMeta(cfg.general.show_item_meta ?? true);
    } catch {}

    refreshTags();

    unlistenClip = await listen<ClipboardChangeEvent>("clipboard-change", (event) => {
      setCurrentClip(event.payload);
      const incomingItem = event.payload.item;

      if (!hasActiveFilters() && incomingItem) {
        setItems((prev) => mergeIncomingItem(prev, incomingItem));
        rememberAvailableContentTypes([incomingItem]);
        setSelectedIndex(0);
        void selectItem(incomingItem);
        return;
      }

      fetchItems(true);
    });

    unlistenMainWindowShown = await listen<string>("main-window-shown", (event) => {
      if (clearInputsOnPanelOpen()) {
        clearPanelInputs();
      }
      window.setTimeout(() => {
        focusSearchInput();
      }, 0);
    });

    document.addEventListener("keydown", handleKeyDown);
    document.addEventListener("pointerdown", handleTagDropdownPointerDown);

    focusSearchOnWindowFocus = () => {
      window.setTimeout(() => {
        focusSearchInput();
      }, 0);
    };

    blurActiveElementOnWindowBlur = () => {
      blurActiveElement();
    };

    window.addEventListener("focus", focusSearchOnWindowFocus);
    window.addEventListener("blur", blurActiveElementOnWindowBlur);
    focusSearchOnWindowFocus();
  });

  onCleanup(() => {
    unlistenClip?.();
    unlistenMainWindowShown?.();
    document.removeEventListener("keydown", handleKeyDown);
    document.removeEventListener("pointerdown", handleTagDropdownPointerDown);
    if (focusSearchOnWindowFocus) {
      window.removeEventListener("focus", focusSearchOnWindowFocus);
    }
    if (blurActiveElementOnWindowBlur) {
      window.removeEventListener("blur", blurActiveElementOnWindowBlur);
    }
  });

  /** 将 Date 格式化为本地时间字符串 "YYYY-MM-DD HH:mm:ss" */
  const fmtLocal = (d: Date): string => {
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  };

  /** 根据预设计算 dateFrom / dateTo */
  const applyDatePreset = (preset: string) => {
    setDatePreset(preset);
    if (preset === "all") {
      setDateFrom(undefined);
      setDateTo(undefined);
      return;
    }
    const now = new Date();
    const endOfDay = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 23, 59, 59);
    let start: Date;
    switch (preset) {
      case "today":
        start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
        break;
      case "yesterday": {
        const y = new Date(now);
        y.setDate(y.getDate() - 1);
        start = new Date(y.getFullYear(), y.getMonth(), y.getDate());
        const yEnd = new Date(y.getFullYear(), y.getMonth(), y.getDate(), 23, 59, 59);
        setDateFrom(fmtLocal(start));
        setDateTo(fmtLocal(yEnd));
        return;
      }
      case "last7days":
        start = new Date(now);
        start.setDate(start.getDate() - 7);
        start.setHours(0, 0, 0, 0);
        break;
      case "last30days":
        start = new Date(now);
        start.setDate(start.getDate() - 30);
        start.setHours(0, 0, 0, 0);
        break;
      case "last90days":
        start = new Date(now);
        start.setDate(start.getDate() - 90);
        start.setHours(0, 0, 0, 0);
        break;
      default:
        setDateFrom(undefined);
        setDateTo(undefined);
        return;
    }
    setDateFrom(fmtLocal(start));
    setDateTo(fmtLocal(endOfDay));
  };

  // 当搜索/过滤条件变化时重新查询
  createEffect(() => {
    keyword();
    typeFilter();
    tagFilter();
    favoriteOnly();
    dateFrom();
    dateTo();
    fetchItems(true);
  });

  createEffect(() => {
    const options = slashFilterOptions();
    setSlashTagIndex((current) => Math.min(current, Math.max(options.length - 1, 0)));
  });

  createEffect(() => {
    if (!showDetailPanelByDefault() && focusArea() === "right") {
      setFocusArea("left");
    }
  });

  return (
    <div class="h-full bg-transparent text-[var(--cb-text)]">
      <div class="cb-main-shell">
        <div data-panel-drag-region class="px-4 pt-3 pb-3 shrink-0 border-b border-[var(--cb-border)]">
          <div class="relative z-10 flex items-center justify-between gap-4">
            <div class="flex flex-1 min-w-0 items-center gap-3">
              <div class="cb-app-mark shrink-0" aria-hidden="true">
                <span
                  class="cb-app-mark-glyph"
                  style={{
                    "-webkit-mask-image": `url(${trayIconUrl})`,
                    "mask-image": `url(${trayIconUrl})`,
                  }}
                />
              </div>
              <div class="relative flex-1 min-w-0">
                <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--cb-text-4)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                </svg>
                <input
                  data-search-input
                  type="text"
                  placeholder={t("search.placeholder")}
                  autocomplete="off"
                  autocorrect="off"
                  autocapitalize="off"
                  spellcheck={false}
                  class="w-full pl-10 pr-3 py-2.5 text-[14px] bg-[var(--cb-bg-elevated)] border border-[var(--cb-border-strong)] rounded-2xl text-[var(--cb-text)] placeholder-[var(--cb-text-4)] shadow-[inset_0_1px_0_rgba(255,255,255,0.35),0_14px_30px_rgba(15,23,42,0.06)] focus:outline-none focus:border-[var(--cb-blue-text)] focus:shadow-[0_0_0_4px_var(--cb-blue-bg)] transition-all"
                  value={searchInputValue()}
                  onInput={(e) => {
                    searchInputSelectionArmed = false;
                    updateSearchInput(e.currentTarget.value);
                  }}
                  onFocus={() => {
                    setIsSearchInputFocused(true);
                    searchInputSelectionArmed = false;
                    setFocusArea("left");
                  }}
                  onBlur={() => {
                    setIsSearchInputFocused(false);
                  }}
                />
                <Show when={showSlashDropdown()}>
                  <div class="absolute left-0 right-0 top-[calc(100%+12px)] z-20 overflow-hidden rounded-[22px] border border-[var(--cb-border-strong)] bg-[color-mix(in_srgb,var(--cb-bg-panel)_88%,transparent)] p-2 shadow-[0_26px_60px_rgba(15,23,42,0.18)] backdrop-blur-xl">
                    <div class="flex items-center justify-between gap-3 px-3 py-2 text-[11px] uppercase tracking-[0.2em] text-[var(--cb-text-4)]">
                      <span>{t("search.commandTags")}</span>
                      <span>{t("search.commandHint")}</span>
                    </div>
                    <Show when={slashFilterOptions().length > 0}>
                      <For each={slashFilterOptions()}>
                        {(option, index) => (
                          <>
                            <Show
                              when={
                                index() === 0 ||
                                slashFilterOptions()[index() - 1]?.group !== option.group
                              }
                            >
                              <div class="px-3 pb-1 pt-3 text-[10px] font-semibold uppercase tracking-[0.18em] text-[var(--cb-text-4)] first:pt-1">
                                {slashGroupLabel(option.group)}
                              </div>
                            </Show>
                            <button
                              class={`flex w-full items-center justify-between gap-3 rounded-2xl px-3 py-2.5 text-left transition-all ${
                                slashTagIndex() === index()
                                  ? "bg-[var(--cb-purple-bg)] text-[var(--cb-text)] shadow-[inset_0_0_0_1px_color-mix(in_srgb,var(--cb-purple-text)_22%,transparent)]"
                                  : "text-[var(--cb-text-2)] hover:bg-[var(--cb-bg-hover)]"
                              }`}
                              onMouseEnter={() => setSlashTagIndex(index())}
                              onMouseDown={(event) => {
                                event.preventDefault();
                                applySlashFilter(option);
                              }}
                            >
                              <div class="flex min-w-0 items-center gap-3">
                                <span
                                  class={`inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-xl text-[12px] font-semibold ${
                                    option.kind === "tag"
                                      ? "bg-[var(--cb-purple-bg)] text-[var(--cb-purple-text)]"
                                      : option.kind === "contentType"
                                        ? "bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)]"
                                        : option.kind === "favorite"
                                          ? "bg-amber-400/10 text-amber-600 [html[data-theme=dark]_&]:text-amber-400/80"
                                          : "bg-[var(--cb-bg-hover)] text-[var(--cb-text-2)]"
                                  }`}
                                >
                                  {option.kind === "tag"
                                    ? "#"
                                    : option.kind === "contentType"
                                      ? t(`contentType.${option.value}`).slice(0, 1)
                                      : option.kind === "favorite"
                                        ? "★"
                                        : "*"}
                                </span>
                                <div class="min-w-0">
                                  <div class="truncate text-[13px] font-medium">{option.label}</div>
                                  <div class="truncate text-[11px] text-[var(--cb-text-4)]">{option.description}</div>
                                </div>
                              </div>
                              <span class="shrink-0 rounded-full border border-[var(--cb-border)] px-2 py-1 text-[10px] uppercase tracking-[0.18em] text-[var(--cb-text-4)]">
                                Enter
                              </span>
                            </button>
                          </>
                        )}
                      </For>
                    </Show>
                  </div>
                </Show>
              </div>
            </div>
            <div class="cb-top-toolbar relative z-20 shrink-0">
              <Show when={showSearchToolbarButtons()}>
                <button class="cb-top-action" onClick={() => props.onOpenPlugins()} title={t("pluginStore.title")}>
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
                  </svg>
                </button>
                <button class="cb-top-action" onClick={() => props.onOpenStats()} title={t("stats.title")}>
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
                  </svg>
                </button>
              </Show>
              <div class="relative shrink-0">
                <button
                  class={`cb-top-action ${datePreset() !== "all" ? "cb-top-action-active" : ""}`}
                  onClick={() => setShowDatePicker(!showDatePicker())}
                  title={t("dateFilter.all")}
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
                  </svg>
                  <Show when={datePreset() !== "all"}>
                    <span class="cb-top-action-dot" />
                  </Show>
                </button>
                <Show when={showDatePicker()}>
                  <DateRangePicker
                    preset={datePreset()}
                    onSelectPreset={(preset) => {
                      applyDatePreset(preset);
                    }}
                    onApplyCustomRange={(from, to) => {
                      setDatePreset("custom");
                      setDateFrom(from);
                      setDateTo(to);
                    }}
                    onClose={() => setShowDatePicker(false)}
                  />
                </Show>
              </div>
              <button class="cb-top-action" onClick={() => props.onOpenSettings()} title={t("common.settings")}>
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
              </button>
            </div>
          </div>

          <div class="relative z-0 mt-3 flex min-w-0 items-center gap-2.5 text-[14px]">
            <div class="flex min-w-0 flex-1 items-center gap-1.5 overflow-x-auto pr-1 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
              <span class="shrink-0 px-1 text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--cb-text-4)]">
                {t("search.filterType")}
              </span>
              <For each={visibleContentTypeKeys()}>
                {(ct) => (
                  <button
                    class={`shrink-0 px-3 py-1.5 text-[13px] rounded-full whitespace-nowrap transition-all border ${
                      typeFilter() === ct.value
                        ? "border-[var(--cb-blue-text)]/20 bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)] shadow-[0_10px_25px_rgba(59,130,246,0.16)]"
                        : "border-transparent bg-[var(--cb-bg-elevated)] text-[var(--cb-text-3)] hover:border-[var(--cb-border-strong)] hover:bg-[var(--cb-bg-hover)] hover:text-[var(--cb-text-2)]"
                    }`}
                    onClick={() => setTypeFilter(typeFilter() === ct.value ? undefined : ct.value)}
                  >
                    {t(`contentType.${ct.key}`)}
                  </button>
                )}
              </For>
            </div>
            <div ref={tagDropdownRef} class="relative w-36 shrink-0">
                <button
                  class={`flex h-8 w-full items-center justify-between gap-2 rounded-full border px-2.5 text-[12px] transition-all ${
                    tagFilter()
                      ? "border-[var(--cb-purple-text)]/18 bg-[var(--cb-purple-bg)] text-[var(--cb-purple-text)]"
                      : "border-[var(--cb-border-light)] bg-[var(--cb-bg-elevated)]/90 text-[var(--cb-text-3)] hover:border-[var(--cb-border-strong)] hover:bg-[var(--cb-bg-hover)] hover:text-[var(--cb-text-2)]"
                  }`}
                  onClick={() => {
                    const next = !tagDropdownOpen();
                    setTagDropdownOpen(next);
                    setTagSearchValue("");
                    if (next) {
                      window.setTimeout(() => tagDropdownInputRef?.focus(), 0);
                    }
                  }}
                  title={t("search.filterTag")}
                >
                  <span class="inline-flex min-w-0 items-center gap-1.5">
                    <span class="text-[11px]">#</span>
                    <span class="truncate">{tagFilter() ? tagFilter() : t("search.filterTag")}</span>
                  </span>
                  <svg
                    class={`h-3 w-3 shrink-0 transition-transform ${tagDropdownOpen() ? "rotate-180" : ""}`}
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                  </svg>
                </button>
                <Show when={tagDropdownOpen()}>
                  <div class="absolute right-0 top-[calc(100%+8px)] z-20 w-60 overflow-hidden rounded-[18px] border border-[var(--cb-border-strong)] bg-[color-mix(in_srgb,var(--cb-bg-panel)_94%,transparent)] p-1.5 shadow-[0_22px_48px_rgba(15,23,42,0.16)] backdrop-blur-xl">
                    <div class="px-1.5 pb-1.5">
                      <div class="relative">
                        <svg class="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[var(--cb-text-4)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-4.35-4.35M10.5 18a7.5 7.5 0 110-15 7.5 7.5 0 010 15z" />
                        </svg>
                        <input
                          ref={tagDropdownInputRef}
                          type="text"
                          value={tagSearchValue()}
                          placeholder={t("tags.tagPlaceholder")}
                          class="w-full rounded-xl border border-[var(--cb-border-light)] bg-[var(--cb-bg-elevated)]/90 py-2 pl-9 pr-3 text-[12px] text-[var(--cb-text)] placeholder-[var(--cb-text-4)] focus:outline-none focus:border-[var(--cb-purple-text)] focus:shadow-[0_0_0_3px_var(--cb-purple-bg)]"
                          onInput={(event) => setTagSearchValue(event.currentTarget.value)}
                          onKeyDown={(event) => {
                            if (event.key === "Escape") {
                              event.preventDefault();
                              closeTagDropdown();
                            } else if (event.key === "Enter") {
                              event.preventDefault();
                              const first = filteredTagOptions()[0];
                              if (first) {
                                applyTagDropdownFilter(first);
                              } else if (!tagSearchValue().trim()) {
                                applyTagDropdownFilter(undefined);
                              }
                            }
                          }}
                        />
                      </div>
                      <div class="mt-1.5 px-1 text-[10px] text-[var(--cb-text-4)]">
                        {filteredTagOptions().length} {t("search.filterTag")}
                      </div>
                    </div>
                    <div class="max-h-52 overflow-y-auto px-1 pb-1">
                      <button
                        class={`flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-[12px] transition-all ${
                          !tagFilter()
                            ? "bg-[var(--cb-purple-bg)] text-[var(--cb-purple-text)]"
                            : "text-[var(--cb-text-2)] hover:bg-[var(--cb-bg-hover)]"
                        }`}
                        onMouseDown={(event) => {
                          event.preventDefault();
                          applyTagDropdownFilter(undefined);
                        }}
                      >
                        <span>{t("contentType.all")}</span>
                        <Show when={!tagFilter()}>
                          <span class="text-[10px]">{t("common.confirm")}</span>
                        </Show>
                      </button>
                      <Show
                        when={filteredTagOptions().length > 0}
                        fallback={
                          <div class="px-3 py-4 text-[12px] text-[var(--cb-text-4)]">
                            {t("search.noResults")}
                          </div>
                        }
                      >
                        <For each={filteredTagOptions()}>
                          {(tag) => (
                            <button
                              class={`mt-1 flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-[12px] transition-all ${
                                tagFilter() === tag
                                  ? "bg-[var(--cb-purple-bg)] text-[var(--cb-purple-text)]"
                                  : "text-[var(--cb-text-2)] hover:bg-[var(--cb-bg-hover)]"
                              }`}
                              onMouseDown={(event) => {
                                event.preventDefault();
                                applyTagDropdownFilter(tag);
                              }}
                            >
                              <span class="truncate">#{tag}</span>
                              <Show when={tagFilter() === tag}>
                                <span class="text-[10px]">{t("common.confirm")}</span>
                              </Show>
                            </button>
                          )}
                        </For>
                      </Show>
                    </div>
                  </div>
                </Show>
            </div>
            <button
              class={`shrink-0 px-3 py-1.5 text-[13px] rounded-full whitespace-nowrap transition-all border ${
                favoriteOnly()
                  ? "border-amber-500/20 bg-amber-400/10 text-amber-600 shadow-[0_10px_24px_rgba(245,158,11,0.16)] [html[data-theme=dark]_&]:text-amber-400/80"
                  : "border-transparent bg-[var(--cb-bg-elevated)] text-[var(--cb-text-3)] hover:border-[var(--cb-border-strong)] hover:bg-[var(--cb-bg-hover)] hover:text-[var(--cb-text-2)]"
              }`}
              onClick={() => setFavoriteOnly(!favoriteOnly())}
            >
              <span class="inline-flex items-center gap-1.5">
                <span>★</span>
                <span>{t("history.pin")}</span>
              </span>
            </button>
          </div>
        </div>
        {/* 主体：左右分栏 */}
        <div class="flex flex-1 overflow-hidden min-h-0">
          {/* 左栏：历史列表 */}
          <HistoryList
            items={items()}
            loading={loading()}
            keyword={keyword()}
            selectedId={selectedId()}
            selectedIndex={selectedIndex()}
            hasMore={hasMore()}
            expanded={!showDetailPanelByDefault()}
            showItemMeta={showItemMeta()}
            currentClipContent={currentClip()?.content}
            onSelectItem={(item: ClipboardHistoryItem, index: number) => {
              setSelectedIndex(index);
              selectItem(item);
              setFocusArea("left");
            }}
            onTogglePin={handleTogglePin}
            onLoadMore={() => fetchItems(false)}
            onDoubleClick={(item: ClipboardHistoryItem) => {
              void confirmHistorySelection(item);
            }}
          />

          {/* 右栏：详情与操作 */}
          <Show when={showDetailPanelByDefault()}>
            <DetailPanel
              item={selectedItem()}
              actions={actions()}
              executing={executing()}
              result={result()}
              error={error()}
              copied={copied()}
              tags={itemTags()}
              focusArea={focusArea()}
              streaming={streaming()}
              streamContent={streamContent()}
              streamThinking={streamThinking()}
              thinking={thinking()}
              onAction={handleAction}
              onCopyResult={handleCopyResult}
              onCopyItem={handleCopyItem}
              onFocusCustomAction={() => setFocusArea("right")}
              onTogglePin={handleTogglePin}
              onDelete={handleDelete}
              onAddTag={handleAddTag}
              onRemoveTag={handleRemoveTag}
              onToggleThinking={() => setThinking((v) => !v)}
              onCustomAction={handleCustomAction}
              onStopStreaming={handleStopStreaming}
              clearInputToken={detailInputClearToken()}
            />
          </Show>
        </div>
      </div>
    </div>
  );
};

/** 将存储中的 content_type 字符串解析回 ContentType 对象 */
function parseContentType(ct: string): ContentType {
  if (ct.startsWith("Code")) {
    const match = ct.match(/Code\("(.+)"\)/);
    return { type: "Code", detail: match?.[1] ?? "" };
  }
  if (ct.startsWith("TableData")) {
    const match = ct.match(/TableData\("(.+)"\)/);
    return { type: "TableData", detail: match?.[1] ?? "" };
  }
  const simple: Record<string, ContentType> = {
    Json: { type: "Json" },
    Yaml: { type: "Yaml" },
    Url: { type: "Url" },
    Email: { type: "Email" },
    PhoneNumber: { type: "PhoneNumber" },
    IdCard: { type: "IdCard" },
    MathExpression: { type: "MathExpression" },
    Image: { type: "Image" },
    FileList: { type: "FileList" },
    PlainText: { type: "PlainText" },
    Unknown: { type: "Unknown" },
  };
  return simple[ct] ?? { type: "Unknown" };
}

function resolveActionInput(item: ClipboardHistoryItem): string | null {
  if (item.content_type === "Image") {
    return item.image_path;
  }
  return item.content;
}

function normalizeContentTypeKey(ct: string): string {
  if (ct.startsWith("Code")) return "Code";
  if (ct.startsWith("TableData")) return "TableData";
  return ct;
}

export default MainLayout;
