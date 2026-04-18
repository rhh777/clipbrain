import { Component, createSignal, For, Show } from "solid-js";
import { completeOnboarding, writeToClipboard } from "../lib/ipc";
import { t } from "../lib/i18n";
import appIconUrl from "../../src-tauri/icons/icon.png";

interface OnboardingGuideProps {
  onComplete: () => void;
}

type Step = "welcome" | "permissions" | "mode" | "done";

const OnboardingGuide: Component<OnboardingGuideProps> = (props) => {
  const [step, setStep] = createSignal<Step>("welcome");
  const [sampleCopied, setSampleCopied] = createSignal(false);

  const sampleJson = `{"name": "ClipBrain", "version": "2.0"}`;

  const handleComplete = async () => {
    try {
      await completeOnboarding();
    } catch (e) {
      console.error("Failed to mark onboarding complete:", e);
    }
    props.onComplete();
  };

  const modes = [
    {
      id: "rules_only",
      nameKey: "onboarding.rulesOnly",
      descKey: "onboarding.rulesOnlyDesc",
      icon: "⚡",
    },
    {
      id: "remote_api",
      nameKey: "onboarding.remoteApi",
      descKey: "onboarding.remoteApiDesc",
      icon: "☁️",
    },
    {
      id: "local_model",
      nameKey: "onboarding.localModel",
      descKey: "onboarding.localModelDesc",
      icon: "🔒",
    },
  ];

  const handleCopySample = async () => {
    try {
      await writeToClipboard(sampleJson);
      setSampleCopied(true);
      window.setTimeout(() => setSampleCopied(false), 2000);
    } catch (e) {
      console.error("Failed to copy onboarding sample:", e);
    }
  };

  return (
    <div class="flex flex-col h-full bg-transparent text-[var(--cb-text)]">
      <div class="flex-1 flex flex-col items-center justify-center px-8 py-12">
        {/* Welcome */}
        <Show when={step() === "welcome"}>
          <div class="text-center space-y-6 max-w-sm animate-fade-in">
            <img
              src={appIconUrl}
              alt="ClipBrain"
              class="mx-auto block h-24 w-24 rounded-[28px] object-cover shadow-[0_18px_44px_rgba(15,23,42,0.14)] select-none"
              draggable={false}
            />
            <h1 class="text-2xl font-semibold text-[var(--cb-text)]">{t("onboarding.welcomeTitle")}</h1>
            <p class="text-[14px] text-[var(--cb-text-3)] leading-relaxed">
              {t("onboarding.welcomeDesc")}
            </p>
            <div class="space-y-2 text-left text-[13px] text-[var(--cb-text-3)]">
              <div class="flex items-center gap-2">
                <span class="text-amber-500">◆</span>
                <span>{t("onboarding.feature1")}</span>
              </div>
              <div class="flex items-center gap-2">
                <span class="text-sky-500">◆</span>
                <span>{t("onboarding.feature2")}</span>
              </div>
              <div class="flex items-center gap-2">
                <span class="text-emerald-500">◆</span>
                <span>{t("onboarding.feature3")}</span>
              </div>
              <div class="flex items-center gap-2">
                <span class="text-violet-500">◆</span>
                <span>{t("onboarding.feature4")}</span>
              </div>
            </div>
            <button
              class="w-full py-2.5 bg-[var(--cb-blue-bg)] hover:opacity-80 text-[var(--cb-blue-text)] rounded-xl text-[14px] font-medium transition-all"
              onClick={() => setStep("permissions")}
            >
              {t("onboarding.startSetup")}
            </button>
          </div>
        </Show>

        {/* Permissions */}
        <Show when={step() === "permissions"}>
          <div class="text-center space-y-6 max-w-sm animate-fade-in">
            <div class="text-5xl">🔐</div>
            <h2 class="text-xl font-semibold text-[var(--cb-text)]">{t("onboarding.permissionsTitle")}</h2>
            <div class="space-y-3 text-left">
              <div class="bg-[var(--cb-bg-card)] rounded-xl p-3 border border-[var(--cb-border)]">
                <h3 class="text-[14px] font-medium text-[var(--cb-text-2)]">{t("onboarding.clipboardAccess")}</h3>
                <p class="text-[13px] text-[var(--cb-text-3)] mt-1">
                  {t("onboarding.clipboardAccessDesc")}
                </p>
              </div>
              <div class="bg-[var(--cb-bg-card)] rounded-xl p-3 border border-[var(--cb-border)]">
                <h3 class="text-[14px] font-medium text-[var(--cb-text-2)]">{t("onboarding.accessibility")}</h3>
                <p class="text-[13px] text-[var(--cb-text-3)] mt-1">
                  {t("onboarding.accessibilityDesc")}
                </p>
              </div>
              <div class="bg-[var(--cb-bg-card)] rounded-xl p-3 border border-[var(--cb-border)]">
                <h3 class="text-[14px] font-medium text-[var(--cb-text-2)]">{t("onboarding.globalHotkey")}</h3>
                <p class="text-[13px] text-[var(--cb-text-3)] mt-1">
                  <kbd class="px-1.5 py-0.5 bg-[var(--cb-bg-hover)] rounded-md text-[12px] text-[var(--cb-text-2)]">⌥⌘C</kbd> {t("onboarding.globalHotkeyDesc")}
                </p>
              </div>
            </div>
            <div class="flex gap-3">
              <button
                class="flex-1 py-2.5 bg-[var(--cb-bg-hover)] hover:opacity-80 rounded-xl text-[14px] text-[var(--cb-text-2)] transition-all"
                onClick={() => setStep("welcome")}
              >
                {t("common.back")}
              </button>
              <button
                class="flex-1 py-2.5 bg-[var(--cb-blue-bg)] hover:opacity-80 text-[var(--cb-blue-text)] rounded-xl text-[14px] font-medium transition-all"
                onClick={() => setStep("mode")}
              >
                {t("onboarding.continue")}
              </button>
            </div>
          </div>
        </Show>

        {/* Mode Selection */}
        <Show when={step() === "mode"}>
          <div class="text-center space-y-5 max-w-sm animate-fade-in">
            <div class="text-5xl">⚙️</div>
            <h2 class="text-xl font-semibold text-[var(--cb-text)]">{t("onboarding.modeTitle")}</h2>
            <p class="text-[13px] text-[var(--cb-text-3)]">{t("onboarding.modeHint")}</p>
            <div class="space-y-2">
              <For each={modes}>
                {(mode, index) => (
                  <div
                    class={`w-full rounded-xl border p-3 text-left transition-all ${
                      index() === 0
                        ? "border-[var(--cb-blue-text)]/25 bg-[var(--cb-blue-bg)]"
                        : "border-[var(--cb-border)] bg-[var(--cb-bg-card)]"
                    }`}
                  >
                   <div class="flex items-center gap-3">
                     <span class="text-2xl">{mode.icon}</span>
                     <div class="min-w-0 flex-1">
                       <div class="flex items-center gap-2">
                         <div class="text-[14px] font-medium text-[var(--cb-text-2)]">{t(mode.nameKey)}</div>
                         <Show when={index() === 0}>
                           <span class="rounded-full bg-[var(--cb-blue-text)]/12 px-2 py-0.5 text-[11px] font-medium text-[var(--cb-blue-text)]">
                             {t("onboarding.modeDefaultBadge")}
                           </span>
                         </Show>
                       </div>
                       <div class="text-[12px] text-[var(--cb-text-3)]">{t(mode.descKey)}</div>
                     </div>
                   </div>
                  </div>
                )}
              </For>
            </div>
            <p class="text-[12px] text-[var(--cb-text-4)]">{t("onboarding.modeSettingsHint")}</p>
            <div class="flex gap-3">
              <button
                class="flex-1 py-2.5 bg-[var(--cb-bg-hover)] hover:opacity-80 rounded-xl text-[14px] text-[var(--cb-text-2)] transition-all"
                onClick={() => setStep("permissions")}
              >
                {t("common.back")}
              </button>
              <button
                class="flex-1 py-2.5 bg-[var(--cb-blue-bg)] hover:opacity-80 text-[var(--cb-blue-text)] rounded-xl text-[14px] font-medium transition-all"
                onClick={() => setStep("done")}
              >
                {t("onboarding.continue")}
              </button>
            </div>
          </div>
        </Show>

        {/* Done */}
        <Show when={step() === "done"}>
          <div class="text-center space-y-6 max-w-sm animate-fade-in">
            <div class="text-5xl">🎉</div>
            <h2 class="text-xl font-semibold text-[var(--cb-text)]">{t("onboarding.doneTitle")}</h2>
            <p class="text-[14px] text-[var(--cb-text-3)] leading-relaxed">
              {t("onboarding.doneDesc")}
            </p>
            <div
              class="cursor-copy rounded-xl border border-[var(--cb-border)] bg-[var(--cb-bg-card)] p-4 text-left transition-all hover:border-[var(--cb-blue-text)]/25 hover:bg-[var(--cb-bg-hover)]"
              onClick={handleCopySample}
              role="button"
              tabindex="0"
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  void handleCopySample();
                }
              }}
            >
              <div class="mb-2 flex items-center justify-between gap-3">
                <p class="text-[13px] text-[var(--cb-text-3)]">{t("onboarding.trySample")}</p>
                <span class={`text-[12px] ${sampleCopied() ? "text-emerald-500" : "text-[var(--cb-text-4)]"}`}>
                  {sampleCopied() ? t("common.copied") : t("onboarding.copySampleHint")}
                </span>
              </div>
              <pre class="overflow-x-auto rounded-lg bg-[var(--cb-bg)] px-3 py-2 text-[13px] text-amber-500 font-mono">
{sampleJson}
              </pre>
            </div>
            <button
              class="w-full py-2.5 bg-[var(--cb-blue-bg)] hover:opacity-80 text-[var(--cb-blue-text)] rounded-xl text-[14px] font-medium transition-all"
              onClick={handleComplete}
            >
              {t("onboarding.startUsing")}
            </button>
          </div>
        </Show>
      </div>

      {/* 步骤指示器 */}
      <div class="flex justify-center gap-2 pb-6">
        {(["welcome", "permissions", "mode", "done"] as Step[]).map((s) => (
          <div
            class={`w-2 h-2 rounded-full transition-all ${
              step() === s ? "bg-[var(--cb-blue-text)]" : "bg-[var(--cb-border)]"
            }`}
          />
        ))}
      </div>
    </div>
  );
};

export default OnboardingGuide;
