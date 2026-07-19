<script lang="ts">
  import type { Snippet } from "svelte";
  import {
    IconBook2,
    IconBox,
    IconBraces,
    IconChevronDown,
    IconCursorText,
    IconHelpCircle,
    IconHistory,
    IconInfoCircle,
    IconKeyboard,
    IconPalette,
    IconServer,
    IconSettings,
    IconShieldLock,
    IconWand,
  } from "@tabler/icons-svelte";

  let {
    sections,
    active,
    theme,
    onselect,
    children,
  }: {
    sections: { id: string; label: string }[];
    active: string;
    theme: string;
    onselect: (id: string) => void;
    children: Snippet;
  } = $props();

  const sectionIcons = {
    providers: IconServer,
    models: IconBox,
    hotkeys: IconKeyboard,
    dictionary: IconBook2,
    snippets: IconBraces,
    styles: IconWand,
    history: IconHistory,
    appearance: IconPalette,
    privacy: IconShieldLock,
    injection: IconCursorText,
    about: IconInfoCircle,
  };

  const groups = [
    { label: "Voice", ids: ["providers", "models", "hotkeys", "injection"] },
    { label: "Writing", ids: ["dictionary", "snippets", "styles", "history"] },
    { label: "System", ids: ["appearance", "privacy", "about"] },
  ];

  function iconFor(id: string) {
    return sectionIcons[id as keyof typeof sectionIcons] ?? IconSettings;
  }

  function navLabelFor(section: { id: string; label: string }) {
    return section.id === "styles" ? "Styles" : section.label;
  }
</script>

<div class="oto-settings" data-theme={theme}>
  <header class="settings-mobile-header">
    <img class="settings-mobile-header__mark" src="/favicon.png" alt="" width="32" height="32" />
    <div class="select-wrap">
      <select
        aria-label="Settings section"
        value={active}
        onchange={(event) => onselect(event.currentTarget.value)}
      >
        {#each sections as section (section.id)}
          <option value={section.id}>{section.label}</option>
        {/each}
      </select>
      <IconChevronDown aria-hidden="true" size={16} stroke={1.7} />
    </div>
  </header>

  <aside class="settings-utility-rail" aria-label="Settings shortcuts">
    <div class="settings-utility-rail__logo" title="Oto settings">
      <img src="/favicon.png" alt="Oto" width="40" height="40" />
    </div>
    <div class="settings-utility-rail__cluster">
      <button class="settings-tool-button" type="button" data-active="true" aria-label="Settings" title="Settings">
        <IconSettings aria-hidden="true" size={20} stroke={1.6} />
      </button>
      <button class="settings-tool-button" type="button" data-active={active === "appearance"} aria-label="Appearance" title="Appearance" onclick={() => onselect("appearance")}>
        <IconPalette aria-hidden="true" size={20} stroke={1.6} />
      </button>
      <button class="settings-tool-button" type="button" data-active={active === "hotkeys"} aria-label="Hotkeys" title="Hotkeys" onclick={() => onselect("hotkeys")}>
        <IconKeyboard aria-hidden="true" size={20} stroke={1.6} />
      </button>
    </div>
    <div class="settings-utility-rail__bottom">
      <button class="settings-tool-button" type="button" data-active={active === "about"} aria-label="About Oto" title="About Oto" onclick={() => onselect("about")}>
        <IconHelpCircle aria-hidden="true" size={20} stroke={1.6} />
      </button>
    </div>
  </aside>

  <aside class="settings-index-rail">
    <div class="settings-brand">
      <div class="settings-brand__copy">
        <span class="settings-brand__name">Oto Settings</span>
        <span class="settings-brand__meta">voice control plane</span>
      </div>
    </div>
    <nav aria-label="Settings sections">
      {#each groups as group (group.label)}
        <div class="settings-nav-group">
          <p class="settings-nav-group__label">{group.label}</p>
          {#each sections.filter((section) => group.ids.includes(section.id)) as section (section.id)}
            {@const SectionIcon = iconFor(section.id)}
            <button
              class="settings-nav-button"
              type="button"
              data-active={active === section.id}
              aria-current={active === section.id ? "page" : undefined}
              onclick={() => onselect(section.id)}
            >
              <SectionIcon aria-hidden="true" size={18} stroke={1.6} />
              <span>{navLabelFor(section)}</span>
            </button>
          {/each}
        </div>
      {/each}
    </nav>
    <div class="settings-index-rail__status">
      <span class="settings-index-rail__status-dot" aria-hidden="true"></span>
      <div>
        <strong>Configuration local</strong>
        <span>Secrets remain in your keyring</span>
      </div>
    </div>
  </aside>

  <main class="settings-main">
    {@render children()}
  </main>
</div>
