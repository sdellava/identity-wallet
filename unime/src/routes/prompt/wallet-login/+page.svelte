<script lang="ts">
  import { onDestroy } from 'svelte';

  import { goto } from '$app/navigation';

  import type { CurrentUserPrompt } from '@bindings/user_prompt/CurrentUserPrompt';

  import { Button, PaddedIcon, TopNavBar } from '$lib/components';
  import { dispatch } from '$lib/dispatcher';
  import { IdentificationBadgeFillIcon, WarningCircleFillIcon } from '$lib/icons';
  import { error, state } from '$lib/stores';

  type IsWalletLoginPrompt<T> = T extends { type: 'wallet-login' } ? T : never;
  type WalletLoginPrompt = IsWalletLoginPrompt<CurrentUserPrompt>;

  const { client_name, origin, session_id } = $state.current_user_prompt as WalletLoginPrompt;

  let loading = false;

  $: hasSeed = Boolean($state.iota_wallet.seed_phrase);
  $: hasDid = Boolean($state.iota_wallet.did);
  $: canApprove = hasSeed && hasDid && !loading;

  error.subscribe((err) => {
    if (err) {
      loading = false;
    }
  });

  onDestroy(async () => {
    dispatch({ type: '[User Flow] Cancel', payload: {} });
  });
</script>

<div class="safe-area-height flex flex-col items-stretch overflow-y-auto bg-silver dark:bg-navy">
  <TopNavBar title="DApp login" on:back={() => history.back()} disabled={loading} class="sticky top-0 z-10" />

  <div class="flex grow flex-col items-center justify-center space-y-6 p-4">
    <PaddedIcon icon={IdentificationBadgeFillIcon} />

    <div class="text-center">
      <p class="text-[22px]/[30px] font-semibold text-slate-700 dark:text-grey">{client_name}</p>
      <p class="pt-[10px] text-sm font-medium text-slate-500">{origin}</p>
    </div>

    <div class="w-full space-y-3 rounded-3xl bg-white p-3 dark:bg-dark">
      <div class="flex w-full items-center rounded-xl bg-amber-50 p-4 text-amber-700 dark:bg-navy">
        <span class="mr-4 h-6 w-6 shrink-0">
          <WarningCircleFillIcon class="h-6 w-6 text-amber-500" />
        </span>
        <div class="flex flex-col">
          <p class="text-[13px]/[24px] font-semibold">Sensitive login request</p>
          <p class="text-[12px]/[20px] font-medium">
            This dApp is requesting your IOTA identity DID and wallet seed. Approve only if you trust this service.
          </p>
        </div>
      </div>

      <div class="space-y-2 rounded-xl bg-silver p-4 text-[12px]/[20px] font-medium text-slate-500 dark:bg-navy">
        <p class="break-all"><span class="font-semibold text-slate-700 dark:text-grey">Session:</span> {session_id}</p>
        <p>
          <span class="font-semibold text-slate-700 dark:text-grey">DID:</span>
          {hasDid ? 'Ready to share' : 'Missing IOTA identity'}
        </p>
        <p>
          <span class="font-semibold text-slate-700 dark:text-grey">Seed:</span>
          {hasSeed ? 'Ready to share' : 'Missing wallet seed'}
        </p>
      </div>
    </div>
  </div>

  <div class="sticky bottom-0 left-0 flex flex-col space-y-[10px] rounded-t-2xl bg-white p-6 dark:bg-dark">
    <Button
      label={loading ? 'Sending login' : 'Approve login'}
      disabled={!canApprove}
      {loading}
      on:click={() => {
        loading = true;
        dispatch({ type: '[IOTA Wallet] Submit wallet login', payload: {} });
      }}
    />
    <Button
      label="Reject"
      variant="secondary"
      disabled={loading}
      on:click={() => {
        dispatch({ type: '[User Flow] Cancel', payload: { redirect: 'me' } });
        goto('/me');
      }}
    />
  </div>
</div>

<style>
  .safe-area-height {
    height: calc(100vh - var(--safe-area-inset-top) - var(--safe-area-inset-bottom));
  }
</style>
