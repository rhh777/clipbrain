import { invoke } from "@tauri-apps/api/core";
import type {
  ContentType,
  ActionDescriptor,
  ActionOutput,
} from "../types/clipboard";

/** 获取剪贴板内容并分类 */
export async function getClipboardContent(): Promise<{
  content: string;
  content_type: ContentType;
}> {
  return invoke("get_clipboard_content");
}

/** 写入剪贴板 */
export async function writeToClipboard(text: string): Promise<void> {
  return invoke("write_to_clipboard", { text });
}

/** 将图片文件写入剪贴板 */
export async function writeImageToClipboard(path: string): Promise<void> {
  return invoke("write_image_to_clipboard", { path });
}

/** 将文件列表写入剪贴板 */
export async function writeFilesToClipboard(paths: string[]): Promise<void> {
  return invoke("write_files_to_clipboard", { paths });
}

/** 触发系统粘贴当前剪贴板内容 */
export async function pasteClipboard(): Promise<void> {
  return invoke("paste_clipboard");
}

/** 恢复之前的应用并触发系统粘贴 */
export async function restorePreviousAppAndPaste(): Promise<void> {
  return invoke("restore_previous_app_and_paste");
}

/** 获取可用操作列表 */
export async function listActions(
  contentType: ContentType,
  locale?: string
): Promise<ActionDescriptor[]> {
  return invoke("list_actions", { contentType, locale });
}

/** 执行操作 */
export async function executeAction(
  actionId: string,
  content: string,
  contentType: ContentType,
  thinking?: boolean
): Promise<ActionOutput> {
  return invoke("execute_action", { actionId, content, contentType, thinking: thinking ?? null });
}

/** 流式执行操作（结果通过 action-stream 事件推送） */
export async function executeActionStream(
  actionId: string,
  content: string,
  contentType: ContentType,
  thinking?: boolean
): Promise<ActionOutput> {
  return invoke("execute_action_stream", { actionId, content, contentType, thinking: thinking ?? null });
}

/** 流式执行自定义操作（用户输入自定义 prompt） */
export async function executeCustomStream(
  content: string,
  prompt: string,
  thinking?: boolean
): Promise<ActionOutput> {
  return invoke("execute_custom_stream", { content, prompt, thinking: thinking ?? null });
}

// --- 模型配置 ---

export interface ModelConfigInput {
  name: string;
  base_url: string;
  api_key: string;
  model: string;
  timeout_secs?: number;
  max_tokens?: number;
}

/** 保存模型配置并注册后端 */
export async function saveModelConfig(config: ModelConfigInput): Promise<string> {
  return invoke("save_model_config", { config });
}

/** 测试模型连接 */
export async function testModelConnection(name: string): Promise<string> {
  return invoke("test_model_connection", { name });
}

/** 列出已配置的后端 */
export async function listModelBackends(): Promise<string[]> {
  return invoke("list_model_backends");
}

/** 检查是否有远程后端 */
export async function hasModelBackend(): Promise<boolean> {
  return invoke("has_model_backend");
}

/** 保存并测试模型配置 */
export async function setupAndTestModel(config: ModelConfigInput): Promise<string> {
  return invoke("setup_and_test_model", { config });
}

export interface ModelConfigOutput {
  name: string;
  base_url: string;
  api_key: string;
  model: string;
  timeout_secs: number;
  max_tokens: number;
  is_active: boolean;
}

/** 列出已保存的模型配置 */
export async function listModelConfigs(): Promise<ModelConfigOutput[]> {
  return invoke("list_model_configs");
}

/** 删除模型配置 */
export async function deleteModelConfig(name: string): Promise<string> {
  return invoke("delete_model_config", { name });
}

/** 切换活跃模型 */
export async function setActiveModel(name: string): Promise<string> {
  return invoke("set_active_model", { name });
}

// --- 历史记录 ---

export interface ClipboardHistoryItem {
  id: number;
  content: string | null;
  image_path: string | null;
  content_type: string;
  source_app: string | null;
  char_count: number | null;
  created_at: string;
  is_pinned: boolean;
  is_sensitive: boolean;
}

export interface FilePreview {
  path: string;
  file_name: string;
  extension: string | null;
  kind: "image" | "text" | "icon";
  data_url: string | null;
  text: string | null;
  truncated: boolean;
  is_dir: boolean;
}

/** 将历史项写回剪贴板，优先写入图片 */
export async function writeHistoryItemToClipboard(item: ClipboardHistoryItem): Promise<void> {
  if (item.image_path) {
    return writeImageToClipboard(item.image_path);
  }
  if (item.content_type === "FileList" && item.content !== null) {
    const paths = item.content
      .split(/\r?\n/)
      .map((path) => path.trim())
      .filter(Boolean);
    if (paths.length > 0) {
      return writeFilesToClipboard(paths);
    }
  }
  if (item.content !== null) {
    return writeToClipboard(item.content);
  }
  throw new Error("Clipboard history item has no content");
}

/** 查询历史记录（分页） */
export async function listHistory(limit?: number, offset?: number): Promise<ClipboardHistoryItem[]> {
  return invoke("list_history", { limit: limit ?? 50, offset: offset ?? 0 });
}

/** 搜索历史记录 */
export async function searchHistory(
  keyword: string,
  contentType?: string,
  limit?: number
): Promise<ClipboardHistoryItem[]> {
  return invoke("search_history", { keyword, contentType: contentType ?? null, limit: limit ?? 50 });
}

/** 删除历史记录 */
export async function deleteHistory(id: number): Promise<void> {
  return invoke("delete_history", { id });
}

/** 切换收藏状态 */
export async function togglePin(id: number): Promise<boolean> {
  return invoke("toggle_pin", { id });
}

/** 清空未收藏的历史记录 */
export async function clearHistory(): Promise<number> {
  return invoke("clear_history");
}

/** 清空未收藏的历史记录，保留最近 retainDays 天的数据（0=全部清空） */
export async function clearHistoryWithRetention(retainDays: number): Promise<number> {
  return invoke("clear_history_with_retention", { retainDays });
}

/** 获取历史记录总数 */
export async function historyCount(): Promise<number> {
  return invoke("history_count");
}

/** 统计未收藏且字节数 >= minBytes 的文本记录，返回 [数量, 总字节数] */
export async function countHistoryOverSize(minBytes: number): Promise<[number, number]> {
  return invoke("count_history_over_size", { minBytes });
}

/** 删除未收藏且字节数 >= minBytes 的文本记录，返回删除数量 */
export async function clearHistoryOverSize(minBytes: number): Promise<number> {
  return invoke("clear_history_over_size", { minBytes });
}

// --- 标签系统 ---

/** 为剪贴板条目添加标签 */
export async function addTag(clipboardId: number, tagName: string): Promise<void> {
  return invoke("add_tag", { clipboardId, tagName });
}

/** 删除剪贴板条目的指定标签 */
export async function removeTag(clipboardId: number, tagName: string): Promise<void> {
  return invoke("remove_tag", { clipboardId, tagName });
}

/** 获取剪贴板条目的所有标签 */
export async function getTags(clipboardId: number): Promise<string[]> {
  return invoke("get_tags", { clipboardId });
}

/** 获取所有已使用的标签（去重） */
export async function listAllTags(): Promise<string[]> {
  return invoke("list_all_tags");
}

/** 根据标签搜索历史记录 ID 列表 */
export async function searchByTag(tagName: string): Promise<number[]> {
  return invoke("search_by_tag", { tagName });
}

/** 高级搜索：关键词 + 内容类型 + 标签 + 日期范围组合筛选 */
export async function searchHistoryAdvanced(
  keyword?: string,
  contentType?: string,
  tag?: string,
  pinnedOnly?: boolean,
  dateFrom?: string,
  dateTo?: string,
  limit?: number,
  offset?: number,
): Promise<ClipboardHistoryItem[]> {
  return invoke("search_history_advanced", {
    keyword: keyword ?? null,
    contentType: contentType ?? null,
    tag: tag ?? null,
    pinnedOnly: pinnedOnly ?? null,
    dateFrom: dateFrom ?? null,
    dateTo: dateTo ?? null,
    limit: limit ?? 50,
    offset: offset ?? 0,
  });
}

// --- 应用图标 ---

/** 获取应用图标的缓存 PNG 路径 */
export async function getAppIcon(appName: string): Promise<string> {
  return invoke("get_app_icon", { appName });
}

/** 获取文件预览（文本/图片/图标） */
export async function getFilePreview(path: string): Promise<FilePreview> {
  return invoke("get_file_preview", { path });
}

// --- 快捷键 ---

export interface QuickActionBinding {
  label: string;
  action_id: string;
  shortcut: string;
  enabled: boolean;
}

export interface AppConfig {
  general: {
    trigger_mode: string;
    capability_mode: string;
    locale: string;
    auto_start: boolean;
    history_limit: number;
    show_detail_panel_by_default: boolean;
    show_search_toolbar_buttons: boolean;
    clear_inputs_on_panel_open: boolean;
    show_item_meta: boolean;
  };
  hotkey: { open_panel: string; quick_translate: string; quick_summarize: string; quick_actions: QuickActionBinding[] };
  popup: { position: string; auto_dismiss_ms: number; max_width: number };
  model: any;
  privacy: any;
}

/** 获取当前配置 */
export async function getConfig(): Promise<AppConfig> {
  return invoke("get_config");
}

/** 更新唤起面板的全局快捷键 */
export async function updateShortcut(oldShortcut: string, newShortcut: string): Promise<void> {
  return invoke("update_shortcut", { oldShortcut, newShortcut });
}

// --- 插件仓库 ---

export interface StorePluginEntry {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;
  download_url: string;
  content_types: string[];
  downloads: number;
}

export interface StoreIndex {
  version: string;
  plugins: StorePluginEntry[];
}

export interface PluginInfo {
  id: string;
  name: string;
  description: string;
  version: string;
  content_types: string[];
}

/** 拉取社区插件仓库索引 */
export async function fetchStoreIndex(): Promise<StoreIndex> {
  return invoke("fetch_store_index");
}

/** 从仓库安装插件 */
export async function installStorePlugin(pluginId: string): Promise<void> {
  return invoke("install_store_plugin", { pluginId });
}

/** 卸载插件 */
export async function uninstallPlugin(pluginId: string): Promise<void> {
  return invoke("uninstall_plugin", { pluginId });
}

/** 获取已安装插件 ID 列表 */
export async function installedPluginIds(): Promise<string[]> {
  return invoke("installed_plugin_ids");
}

/** 列出已安装插件详情 */
export async function listPlugins(): Promise<PluginInfo[]> {
  return invoke("list_plugins");
}

/** 重新加载插件 */
export async function reloadPlugins(): Promise<number> {
  return invoke("reload_plugins");
}

// --- 图片 ---

/** 读取图片文件并返回 base64 data URL */
export async function readImageBase64(path: string): Promise<string> {
  return invoke("read_image_base64", { path });
}

// --- 统计 ---

export interface ActionUsageStat {
  action_id: string;
  display_name: string;
  count: number;
  total_duration_ms: number;
}

export interface DailyStat {
  date: string;
  count: number;
}

export interface ActionStats {
  total_count: number;
  total_duration_ms: number;
  top_actions: ActionUsageStat[];
  daily_trend: DailyStat[];
}

/** 获取操作统计概览 */
export async function getStats(locale?: string): Promise<ActionStats> {
  return invoke("get_stats", { locale: locale ?? null });
}

// --- 首次引导 ---

/** 保存配置 */
export async function saveConfig(config: AppConfig): Promise<void> {
  return invoke("save_config", { config });
}

/** 执行快捷操作（读剪贴板 → 执行 → 写回） */
export async function executeQuickAction(actionId: string): Promise<{ success: boolean; action_id: string; message: string }> {
  return invoke("execute_quick_action", { actionId });
}

/** 检测是否为首次启动 */
export async function isFirstLaunch(): Promise<boolean> {
  return invoke("is_first_launch");
}

/** 标记引导已完成 */
export async function completeOnboarding(): Promise<void> {
  return invoke("complete_onboarding");
}
