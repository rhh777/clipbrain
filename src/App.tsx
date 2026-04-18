import { Component, createSignal, Match, Show, Switch, onMount, onCleanup } from "solid-js";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import MainLayout from "./components/MainLayout";
import SettingsPage from "./components/SettingsPage";
import StatsPanel from "./components/StatsPanel";
import PluginStore from "./components/PluginStore";
import OnboardingGuide from "./components/OnboardingGuide";
import { isFirstLaunch } from "./lib/ipc";
import "./lib/theme";

type Page = "panel" | "settings" | "stats" | "plugins" | "onboarding";

const App: Component = () => {
  const [page, setPage] = createSignal<Page | null>(null);

  let unlistenNav: UnlistenFn | undefined;

  onMount(async () => {
    // 监听托盘菜单导航事件
    unlistenNav = await listen("navigate", (event) => {
      const target = event.payload as string;
      if (["settings", "panel", "stats", "plugins"].includes(target)) {
        setPage(target as Page);
      }
    });

    // 检测首次启动。初始化完成前不渲染页面，避免先闪出主面板再切到引导。
    try {
      const firstLaunch = await isFirstLaunch();
      setPage((current) => current ?? (firstLaunch ? "onboarding" : "panel"));
    } catch (e) {
      console.error("Failed to detect first launch:", e);
      setPage((current) => current ?? "panel");
    }
  });

  onCleanup(() => {
    unlistenNav?.();
  });

  const handleWindowDrag = async (event: MouseEvent) => {
    if (event.button !== 0) return;
    if (!(event.currentTarget instanceof HTMLElement)) return;

    const target = event.target;
    const dragRegion =
      target instanceof Element ? target.closest("[data-panel-drag-region]") : null;
    if (
      target instanceof Element &&
      target.closest('[data-no-panel-drag], button, input, textarea, select, a, [role="button"], [contenteditable="true"], label')
    ) {
      return;
    }

    if (!dragRegion) {
      const bounds = event.currentTarget.getBoundingClientRect();
      if (event.clientY - bounds.top > 72) return;
    }

    event.preventDefault();
    try {
      await invoke("start_panel_drag");
    } catch {
      await getCurrentWindow().startDragging();
    }
  };

  return (
    <div class="cb-window">
      <div class="cb-window-body" onMouseDown={handleWindowDrag}>
        <div class="cb-window-content">
          <Show
            when={page() !== null}
            fallback={<div class="cb-page-shell"><div class="cb-pane h-full overflow-hidden" /></div>}
          >
            <Switch
              fallback={
                <MainLayout
                  onOpenSettings={() => setPage("settings")}
                  onOpenStats={() => setPage("stats")}
                  onOpenPlugins={() => setPage("plugins")}
                />
              }
            >
              <Match when={page() === "onboarding"}>
                <div class="cb-page-shell">
                  <div class="cb-pane h-full overflow-hidden">
                    <OnboardingGuide onComplete={() => setPage("panel")} />
                  </div>
                </div>
              </Match>
              <Match when={page() === "settings"}>
                <div class="cb-page-shell">
                  <div class="cb-pane h-full overflow-hidden">
                    <SettingsPage onBack={() => setPage("panel")} />
                  </div>
                </div>
              </Match>
              <Match when={page() === "stats"}>
                <div class="cb-page-shell">
                  <div class="cb-pane h-full overflow-hidden">
                    <StatsPanel onBack={() => setPage("panel")} />
                  </div>
                </div>
              </Match>
              <Match when={page() === "plugins"}>
                <div class="cb-page-shell">
                  <div class="cb-pane h-full overflow-hidden">
                    <PluginStore onBack={() => setPage("panel")} />
                  </div>
                </div>
              </Match>
            </Switch>
          </Show>
        </div>
      </div>
    </div>
  );
};

export default App;
