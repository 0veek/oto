<script lang="ts">
  import { onMount } from "svelte";
  import type { Snippet } from "svelte";
  import {
    IconBook2,
    IconBox,
    IconContrast2,
    IconCursorText,
    IconCut,
    IconHandStop,
    IconHistory,
    IconInfoCircle,
    IconKeyboard,
    IconSearch,
    IconShieldCheck,
    IconTypography,
    IconWaveSine,
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

  let query = $state("");
  let searchInput: HTMLInputElement;

  const sectionIcons = {
    providers: IconWaveSine,
    models: IconBox,
    hotkeys: IconKeyboard,
    injection: IconCursorText,
    dictionary: IconBook2,
    snippets: IconCut,
    styles: IconTypography,
    history: IconHistory,
    permissions: IconShieldCheck,
    appearance: IconContrast2,
    privacy: IconHandStop,
    about: IconInfoCircle,
  };

  const groups = [
    { label: "Voice", ids: ["providers", "models", "hotkeys", "injection"] },
    { label: "Writing", ids: ["dictionary", "snippets", "styles", "history"] },
    { label: "System", ids: ["permissions", "appearance", "privacy", "about"] },
  ];

  function iconFor(id: string) {
    return sectionIcons[id as keyof typeof sectionIcons] ?? IconInfoCircle;
  }

  function navLabelFor(section: { id: string; label: string }) {
    if (section.id === "styles") return "Styles";
    if (section.id === "privacy") return "Privacy";
    return section.label;
  }

  function visibleSections(ids: string[]) {
    const normalizedQuery = query.trim().toLocaleLowerCase();
    return sections.filter((section) => {
      if (!ids.includes(section.id)) return false;
      return !normalizedQuery || navLabelFor(section).toLocaleLowerCase().includes(normalizedQuery);
    });
  }

  onMount(() => {
    const focusSearch = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLocaleLowerCase() === "f") {
        event.preventDefault();
        searchInput?.focus();
        searchInput?.select();
      }
    };
    window.addEventListener("keydown", focusSearch);
    return () => window.removeEventListener("keydown", focusSearch);
  });
</script>

<div class="oto-settings" data-theme={theme}>
  <header class="settings-mobile-header">
    <img class="settings-mobile-header__mark" src="/favicon.png" alt="" width="32" height="32" />
    <div class="settings-mobile-header__copy">
      <strong>Oto</strong>
      <span>{sections.find((section) => section.id === active)?.label ?? "Settings"}</span>
    </div>
    <select
      aria-label="Settings section"
      value={active}
      onchange={(event) => onselect(event.currentTarget.value)}
    >
      {#each sections as section (section.id)}
        <option value={section.id}>{section.label}</option>
      {/each}
    </select>
  </header>

  <aside class="settings-index-rail">
    <label class="settings-search">
      <IconSearch aria-hidden="true" size={19} stroke={1.8} />
      <input bind:this={searchInput} bind:value={query} type="search" placeholder="Search" aria-label="Search settings" />
      <kbd>⌘ F</kbd>
    </label>

    <nav aria-label="Settings sections">
      {#each groups as group (group.label)}
        {@const matches = visibleSections(group.ids)}
        {#if matches.length}
          <div class="settings-nav-group">
            <p class="settings-nav-group__label">{group.label}</p>
            {#each matches as section (section.id)}
              {@const SectionIcon = iconFor(section.id)}
              <button
                class="settings-nav-button"
                type="button"
                data-active={active === section.id}
                aria-current={active === section.id ? "page" : undefined}
                onclick={() => onselect(section.id)}
              >
                <SectionIcon aria-hidden="true" size={20} stroke={1.65} />
                <span>{navLabelFor(section)}</span>
              </button>
            {/each}
          </div>
        {/if}
      {/each}

      {#if query.trim() && groups.every((group) => visibleSections(group.ids).length === 0)}
        <p class="settings-search__empty">No settings found.</p>
      {/if}
    </nav>
  </aside>

  <main class="settings-main">
    {@render children()}
  </main>
</div>
