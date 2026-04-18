import { Component, createSignal, For, Show, onMount } from "solid-js";
import {
  fetchStoreIndex,
  installStorePlugin,
  uninstallPlugin,
  installedPluginIds,
  reloadPlugins,
  type StorePluginEntry,
} from "../lib/ipc";
import { t } from "../lib/i18n";

interface PluginStoreProps {
  onBack: () => void;
}

const PluginStore: Component<PluginStoreProps> = (props) => {
  const [plugins, setPlugins] = createSignal<StorePluginEntry[]>([]);
  const [installedIds, setInstalledIds] = createSignal<Set<string>>(new Set());
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [installing, setInstalling] = createSignal<string | null>(null);
  const [message, setMessage] = createSignal<{ text: string; ok: boolean } | null>(null);

  const loadData = async () => {
    setLoading(true);
    setError(null);
    try {
      const [index, ids] = await Promise.all([
        fetchStoreIndex(),
        installedPluginIds(),
      ]);
      setPlugins(index.plugins);
      setInstalledIds(new Set(ids));
    } catch (e: any) {
      setError(e?.toString() ?? t("pluginStore.fetchError"));
    } finally {
      setLoading(false);
    }
  };

  onMount(loadData);

  const handleInstall = async (pluginId: string) => {
    setInstalling(pluginId);
    setMessage(null);
    try {
      await installStorePlugin(pluginId);
      await reloadPlugins();
      const ids = await installedPluginIds();
      setInstalledIds(new Set(ids));
      setMessage({ text: t("pluginStore.installSuccess"), ok: true });
    } catch (e: any) {
      setMessage({ text: e?.toString() ?? "Install failed", ok: false });
    } finally {
      setInstalling(null);
    }
  };

  const handleUninstall = async (pluginId: string) => {
    setInstalling(pluginId);
    setMessage(null);
    try {
      await uninstallPlugin(pluginId);
      await reloadPlugins();
      const ids = await installedPluginIds();
      setInstalledIds(new Set(ids));
      setMessage({ text: t("pluginStore.uninstallSuccess"), ok: true });
    } catch (e: any) {
      setMessage({ text: e?.toString() ?? "Uninstall failed", ok: false });
    } finally {
      setInstalling(null);
    }
  };

  const isInstalled = (id: string) => installedIds().has(id);

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
        <h1 class="text-[14px] font-medium text-[var(--cb-text-2)]">{t("pluginStore.title")}</h1>
      </header>

      {/* 消息提示 */}
      <Show when={message()}>
        {(msg) => (
          <div class={`mx-4 mt-3 px-3 py-2 rounded-lg text-[13px] ${
            msg().ok
              ? "bg-[var(--cb-emerald-bg)] text-[var(--cb-emerald-text)]"
              : "bg-[var(--cb-red-bg)] text-[var(--cb-red-text)]"
          }`}>
            {msg().text}
          </div>
        )}
      </Show>

      {/* 内容区 */}
      <div class="flex-1 overflow-y-auto p-4 space-y-3">
        {/* 加载中 */}
        <Show when={loading()}>
          <div class="flex items-center justify-center py-16 text-[var(--cb-text-3)]">
            <div class="w-5 h-5 border-2 border-[var(--cb-text-3)] border-t-transparent rounded-full animate-spin" />
          </div>
        </Show>

        {/* 错误 */}
        <Show when={error()}>
          {(err) => (
            <div class="flex flex-col items-center justify-center py-12 text-[var(--cb-text-3)] space-y-3">
              <svg class="w-10 h-10 opacity-20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
              </svg>
              <p class="text-[13px]">{err()}</p>
              <button
                class="px-3 py-1.5 text-[12px] rounded-lg bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)] hover:opacity-80 transition-all"
                onClick={loadData}
              >
                Retry
              </button>
            </div>
          )}
        </Show>

        {/* 插件列表 */}
        <Show when={!loading() && !error() && plugins().length > 0}>
          <div class="space-y-2">
            <h2 class="text-[12px] font-medium text-[var(--cb-text-3)] tracking-wider">{t("pluginStore.available")}</h2>
            <For each={plugins()}>
              {(plugin) => {
                const installed = () => isInstalled(plugin.id);
                const busy = () => installing() === plugin.id;

                return (
                  <div class="p-3 bg-[var(--cb-bg-card)] border border-[var(--cb-border)] rounded-xl">
                    <div class="flex items-start justify-between gap-3">
                      <div class="flex-1 min-w-0">
                        <div class="flex items-center gap-2">
                          <span class="text-[14px] font-medium text-[var(--cb-text)]">{plugin.name}</span>
                          <span class="text-[10px] px-1.5 py-0 rounded bg-[var(--cb-bg-hover)] text-[var(--cb-text-3)]">
                            v{plugin.version}
                          </span>
                          <Show when={installed()}>
                            <span class="text-[10px] px-1.5 py-0 rounded bg-[var(--cb-emerald-bg)] text-[var(--cb-emerald-text)]">
                              {t("pluginStore.installed_badge")}
                            </span>
                          </Show>
                        </div>
                        <p class="text-[12px] text-[var(--cb-text-3)] mt-1 line-clamp-2">{plugin.description}</p>
                        <div class="flex items-center gap-3 mt-1.5 text-[11px] text-[var(--cb-text-4)]">
                          <span>{t("pluginStore.by")} {plugin.author}</span>
                          <Show when={plugin.content_types.length > 0}>
                            <span class="flex gap-1">
                              <For each={plugin.content_types.slice(0, 3)}>
                                {(ct) => (
                                  <span class="px-1 rounded bg-[var(--cb-bg-hover)]">{ct}</span>
                                )}
                              </For>
                            </span>
                          </Show>
                        </div>
                      </div>
                      <div class="shrink-0">
                        <Show when={installed()} fallback={
                          <button
                            class="px-3 py-1.5 text-[12px] rounded-lg bg-[var(--cb-blue-bg)] text-[var(--cb-blue-text)] hover:opacity-80 transition-all disabled:opacity-40"
                            onClick={() => handleInstall(plugin.id)}
                            disabled={busy()}
                          >
                            {busy() ? t("pluginStore.installing") : t("pluginStore.install")}
                          </button>
                        }>
                          <button
                            class="px-3 py-1.5 text-[12px] rounded-lg bg-[var(--cb-red-bg)] text-[var(--cb-red-text)] hover:opacity-80 transition-all disabled:opacity-40"
                            onClick={() => handleUninstall(plugin.id)}
                            disabled={busy()}
                          >
                            {t("pluginStore.uninstall")}
                          </button>
                        </Show>
                      </div>
                    </div>
                  </div>
                );
              }}
            </For>
          </div>
        </Show>

        {/* 空状态 */}
        <Show when={!loading() && !error() && plugins().length === 0}>
          <div class="flex flex-col items-center justify-center py-12 text-[var(--cb-text-3)] space-y-2">
            <svg class="w-12 h-12 opacity-20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4" />
            </svg>
            <p class="text-[13px]">{t("pluginStore.noPlugins")}</p>
          </div>
        </Show>
      </div>
    </div>
  );
};

export default PluginStore;
