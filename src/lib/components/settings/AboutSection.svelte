<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  let version = $state<string>("…");
  let versionError = $state<string | null>(null);

  onMount(async () => {
    try {
      version = await invoke<string>("get_app_version");
    } catch (e) {
      version = "0.1.0";
      versionError = String(e);
    }
  });
</script>

<section class="space-y-6">
  <header>
    <h2 class="text-xl font-semibold tracking-tight">About</h2>
    <p class="mt-1 text-sm text-slate-400">
      Oto is system-wide AI voice dictation for Linux.
    </p>
  </header>

  <div
    class="space-y-5 rounded-2xl border border-white/10 bg-white/[0.04] p-6 shadow-xl backdrop-blur-xl"
  >
    <div class="flex items-baseline justify-between gap-4">
      <div>
        <div class="text-sm font-medium text-slate-300">Version</div>
        <div class="mt-1 font-mono text-lg text-white">{version}</div>
        {#if versionError}
          <p class="mt-1 text-xs text-slate-500">
            Could not read package version from Tauri ({versionError}).
          </p>
        {/if}
      </div>
      <div class="text-right text-xs text-slate-500">dev.oto.app</div>
    </div>

    <div class="space-y-2 border-t border-white/10 pt-4">
      <h3 class="text-sm font-medium text-slate-200">Privacy</h3>
      <p class="text-sm leading-relaxed text-slate-400">
        Cloud STT sends audio to your chosen provider; Local Whisper keeps transcription
        on-device. Polish and Command Mode send text to the configured LLM. API keys stay in
        the OS keyring. History is local, and sync only runs against the endpoint you choose.
        Oto operates no intermediary cloud.
      </p>
    </div>

    <div class="space-y-2 border-t border-white/10 pt-4 text-xs leading-relaxed text-slate-500">
      <p>
        Config (no secrets) lives under your XDG config dir
        <code class="rounded bg-white/5 px-1">~/.config/oto/config.json</code>
        (path may vary by distro).
      </p>
      <p>
        Design notes:
        <code class="rounded bg-white/5 px-1"
          >docs/superpowers/specs/2026-07-19-oto-design.md</code
        >
      </p>
    </div>
  </div>
</section>
