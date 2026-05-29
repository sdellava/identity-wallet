<script lang="ts">
  import { tick } from 'svelte';
  import { writable } from 'svelte/store';
  import { goto } from '$app/navigation';
  import { open } from '@tauri-apps/plugin-shell';

  import { ActionSheet, TopNavBar } from '$lib/components';
  import { dispatch } from '$lib/dispatcher';
  import {
    ArrowCounterClockwiseBoldIcon,
    ArrowSquareOutBoldIcon,
    CodeRegularIcon,
    TrashRegularIcon,
    WarningCircleFillIcon,
  } from '$lib/icons';
  import { state } from '$lib/stores';

  import { buildIotaExplorerSearchLink } from '../../activity/utils';

  let rotating = false;
  let destroying = false;
  let loadingDocument = false;
  let showDocument = false;
  let resultTitle = '';
  let resultDescription = '';
  const resultOpen = writable(false);
  const destroyConfirmOpen = writable(false);

  const showResult = (title: string, description: string) => {
    resultTitle = title;
    resultDescription = description;
    resultOpen.set(true);
  };

  const rotateKeys = async () => {
    rotating = true;
    await dispatch({ type: '[IOTA Wallet] Rotate identity keys', payload: {} });
    await tick();
    rotating = false;
    if ($state.iota_wallet.last_error) {
      showResult('Key rotation failed', $state.iota_wallet.last_error);
    } else {
      showResult('Keys rotated', 'The identity keys were rotated successfully.');
    }
  };

  const destroyIdentity = async () => {
    destroying = true;
    destroyConfirmOpen.set(false);
    await dispatch({ type: '[IOTA Wallet] Destroy identity', payload: {} });
    await tick();
    destroying = false;
    if ($state.iota_wallet.last_error) {
      showResult('Identity destruction failed', $state.iota_wallet.last_error);
    } else {
      showResult('Identity destroyed', 'The identity controller cap was removed and destroyed successfully.');
    }
  };

  const toggleDidDocument = async () => {
    if (showDocument) {
      showDocument = false;
      return;
    }

    loadingDocument = true;
    await dispatch({ type: '[IOTA Wallet] Validate identity', payload: {} });
    await tick();
    loadingDocument = false;
    if ($state.iota_wallet.last_error) {
      showResult('DID document refresh failed', $state.iota_wallet.last_error);
      return;
    }
    if ($state.iota_wallet.identity_validation_error) {
      showResult('DID document refresh failed', $state.iota_wallet.identity_validation_error);
      return;
    }
    showDocument = true;
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

    <section class="grid grid-cols-3 gap-3">
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
        class="flex min-h-24 flex-col items-start justify-between rounded-xl border border-slate-200 bg-white p-4 text-left disabled:opacity-50 dark:border-slate-600 dark:bg-dark"
        onclick={toggleDidDocument}
        disabled={loadingDocument}
      >
        <CodeRegularIcon class="size-5 text-primary" />
        <span class="text-[13px]/[18px] font-semibold text-slate-800 dark:text-grey">
          {loadingDocument ? 'Loading document' : showDocument ? 'Hide document' : 'DID document'}
        </span>
      </button>
      <button
        class="flex min-h-24 flex-col items-start justify-between rounded-xl border border-rose-200 bg-white p-4 text-left text-rose-500 disabled:opacity-50 dark:border-rose-900/60 dark:bg-dark"
        onclick={() => destroyConfirmOpen.set(true)}
        disabled={destroying}
      >
        <TrashRegularIcon class="size-5" />
        <span class="text-[13px]/[18px] font-semibold">
          {destroying ? 'Destroying' : 'Destroy identity'}
        </span>
      </button>
    </section>

    {#if showDocument}
      <section class="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-600 dark:bg-dark">
        <p class="text-base font-semibold text-slate-800 dark:text-grey">DID document</p>
        <pre class="mt-3 max-h-[420px] overflow-auto whitespace-pre-wrap break-all rounded-lg bg-silver p-3 font-mono text-[11px]/[16px] text-slate-800 dark:bg-navy dark:text-grey">{didDocument}</pre>
      </section>
    {/if}

    <ActionSheet
      titleText="Destroy identity"
      descriptionText="This removes the controller cap from the identity and destroys it on IOTA."
      open={destroyConfirmOpen}
    >
      <WarningCircleFillIcon slot="icon" class="mb-3 size-10 text-rose-500" />
      <div slot="content" class="flex w-full flex-col gap-3 pt-5">
        <button
          class="h-12 w-full rounded-xl bg-rose-500 px-4 py-2 text-[13px]/[24px] font-semibold text-white disabled:opacity-50"
          onclick={destroyIdentity}
          disabled={destroying}
        >
          {destroying ? 'Destroying' : 'Destroy identity'}
        </button>
        <button
          class="h-12 w-full rounded-xl border border-slate-200 bg-white px-4 py-2 text-[13px]/[24px] font-semibold text-slate-800 dark:border-slate-600 dark:bg-dark dark:text-grey"
          onclick={() => destroyConfirmOpen.set(false)}
        >
          Cancel
        </button>
      </div>
    </ActionSheet>
  {/if}
</div>

<ActionSheet titleText={resultTitle} descriptionText={resultDescription} open={resultOpen}>
  <div slot="content" class="flex w-full flex-col pt-5">
    <button
      class="h-12 w-full rounded-xl bg-primary px-4 py-2 text-[13px]/[24px] font-semibold text-white dark:text-dark"
      onclick={() => resultOpen.set(false)}
    >
      OK
    </button>
  </div>
</ActionSheet>
