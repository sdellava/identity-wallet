<script lang="ts">
  import { goto } from '$app/navigation';
  import { open } from '@tauri-apps/plugin-shell';

  import { TopNavBar } from '$lib/components';
  import { dispatch } from '$lib/dispatcher';
  import { ArrowCounterClockwiseBoldIcon, ArrowSquareOutBoldIcon, CodeRegularIcon } from '$lib/icons';
  import { state } from '$lib/stores';

  import { buildIotaExplorerSearchLink } from '../../activity/utils';

  let rotating = false;
  let showDocument = false;

  const rotateKeys = async () => {
    rotating = true;
    await dispatch({ type: '[IOTA Wallet] Rotate identity keys', payload: {} });
    rotating = false;
  };

  const openExplorer = async () => {
    if (!$state.iota_wallet.did) return;
    await open(buildIotaExplorerSearchLink($state.iota_wallet.did));
  };

  $: didDocument = $state.iota_wallet.did_document ?? 'DID document not loaded yet.';
</script>

<TopNavBar on:back={() => history.back()} title="European Verifiable Identity" class="sticky top-0 z-10" />

<div class="flex min-h-full flex-col gap-4 bg-silver px-4 py-5 dark:bg-navy">
  {#if !$state.iota_wallet.did}
    <section class="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-600 dark:bg-dark">
      <p class="text-base font-semibold text-slate-800 dark:text-grey">No identity created</p>
      <p class="mt-1 text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">
        Create your European Verifiable Identity from the Add page.
      </p>
      <button
        class="mt-4 rounded-lg bg-primary px-3 py-2 text-[12px]/[16px] font-semibold text-white dark:text-dark"
        onclick={() => goto('/me/add')}
      >
        Create
      </button>
    </section>
  {:else}
    <section class="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-600 dark:bg-dark">
      <div class="flex items-center justify-between gap-3">
        <div>
          <p class="text-base font-semibold text-slate-800 dark:text-grey">Identity</p>
          <p class="text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">
            {$state.iota_wallet.network}
          </p>
        </div>
        <button
          class="flex size-10 items-center justify-center rounded-lg border border-slate-200 text-slate-700 dark:border-slate-600 dark:text-grey"
          aria-label="Open in IOTA explorer"
          title="Open in IOTA explorer"
          onclick={openExplorer}
        >
          <ArrowSquareOutBoldIcon class="size-5" />
        </button>
      </div>
      <p class="mt-4 font-mono text-[11px]/[16px] break-all text-slate-800 dark:text-grey">
        {$state.iota_wallet.did}
      </p>
    </section>

    <section class="grid grid-cols-2 gap-3">
      <button
        class="flex min-h-24 flex-col items-start justify-between rounded-xl border border-slate-200 bg-white p-4 text-left disabled:opacity-50 dark:border-slate-600 dark:bg-dark"
        onclick={rotateKeys}
        disabled={rotating}
      >
        <ArrowCounterClockwiseBoldIcon class="size-5 text-primary" />
        <span class="text-[13px]/[18px] font-semibold text-slate-800 dark:text-grey">
          {rotating ? 'Rotating keys' : 'Rotate keys'}
        </span>
      </button>
      <button
        class="flex min-h-24 flex-col items-start justify-between rounded-xl border border-slate-200 bg-white p-4 text-left dark:border-slate-600 dark:bg-dark"
        onclick={() => (showDocument = !showDocument)}
      >
        <CodeRegularIcon class="size-5 text-primary" />
        <span class="text-[13px]/[18px] font-semibold text-slate-800 dark:text-grey">
          {showDocument ? 'Hide document' : 'DID document'}
        </span>
      </button>
    </section>

    {#if showDocument}
      <section class="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-600 dark:bg-dark">
        <p class="text-base font-semibold text-slate-800 dark:text-grey">DID document</p>
        <pre class="mt-3 max-h-[420px] overflow-auto whitespace-pre-wrap break-all rounded-lg bg-silver p-3 font-mono text-[11px]/[16px] text-slate-800 dark:bg-navy dark:text-grey">{didDocument}</pre>
      </section>
    {/if}

    {#if $state.iota_wallet.identity_controller_cap}
      <section class="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-600 dark:bg-dark">
        <p class="text-base font-semibold text-slate-800 dark:text-grey">Controller cap</p>
        <p class="mt-3 font-mono text-[11px]/[16px] break-all text-slate-800 dark:text-grey">
          {$state.iota_wallet.identity_controller_cap}
        </p>
      </section>
    {/if}

    {#if $state.iota_wallet.identity_rotation_status}
      <p class="rounded-xl bg-white p-4 text-[12px]/[18px] font-medium text-slate-600 dark:bg-dark dark:text-slate-300">
        Key rotation: {$state.iota_wallet.identity_rotation_status}
      </p>
    {/if}
  {/if}

  {#if $state.iota_wallet.last_error}
    <p class="rounded-xl bg-white p-4 text-[12px]/[18px] font-medium text-rose-500 dark:bg-dark">
      {$state.iota_wallet.last_error}
    </p>
  {/if}
</div>
