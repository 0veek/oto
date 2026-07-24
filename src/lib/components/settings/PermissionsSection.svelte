<script lang="ts">
  import { IconCheck, IconExternalLink, IconMicrophone, IconPointer } from "@tabler/icons-svelte";

  let {
    onselect,
  }: {
    /** Navigate to another settings section without a full page remount. */
    onselect?: (id: string) => void;
  } = $props();
</script>

<section class="space-y-6">
  <header>
    <h2 class="text-xl font-semibold tracking-tight">Permissions</h2>
    <p class="mt-1 text-sm text-slate-400">
      Access Oto needs to capture speech and insert text into the focused app.
    </p>
  </header>

  <div class="permissions-list settings-panel">
    <div class="permission-row">
      <span class="permission-row__icon"><IconMicrophone aria-hidden="true" size={19} stroke={1.7} /></span>
      <div>
        <strong>Microphone</strong>
        <span>Required while Oto is listening.</span>
      </div>
      <span class="permission-row__status"><IconCheck aria-hidden="true" size={16} /> Managed by system</span>
    </div>
    <div class="permission-row">
      <span class="permission-row__icon"><IconPointer aria-hidden="true" size={19} stroke={1.7} /></span>
      <div>
        <strong>Accessibility and input</strong>
        <span>Lets Oto place dictated text in other applications.</span>
      </div>
      <button
        type="button"
        class="permission-row__link"
        onclick={() => onselect?.("injection")}
      >
        Review injection <IconExternalLink aria-hidden="true" size={15} />
      </button>
    </div>
  </div>
</section>
