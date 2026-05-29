<script lang="ts">
  import type { Component } from 'svelte';

  import { goto } from '$app/navigation';
  import LL from '$i18n/i18n-svelte';
  import type { SVGAttributes } from 'svelte/elements';

  import { TopNavBar } from '$lib/components';
  import { dispatch } from '$lib/dispatcher';
  import {
    CaretRightBoldIcon,
    EnvelopeOpenFillIcon,
    HouseFillIcon,
    IdentificationBadgeFillIcon,
    SealCheckFillIcon,
  } from '$lib/icons';
  import { state as appState } from '$lib/stores';

  type Data = {
    title: string;
    description: string;
    icon: Component<SVGAttributes<SVGSVGElement>>;
    link: string;
    disabled?: boolean;
  };

  const data: Data[] = [
    {
      title: $LL.ADD_CREDENTIALS.PROFILE.TITLE(),
      description: $LL.ADD_CREDENTIALS.PROFILE.DESCRIPTION(),
      icon: IdentificationBadgeFillIcon,
      link: '/me/add/profile/info',
    },
    {
      title: $LL.ADD_CREDENTIALS.EMAIL.TITLE(),
      description: $LL.ADD_CREDENTIALS.EMAIL.DESCRIPTION(),
      icon: EnvelopeOpenFillIcon,
      // If there is an active verification case, skip the /info page to avoid "flickering".
      link: $appState.verified_data.email_verification ? '/me/add/email' : '/me/add/email/info',
    },
    {
      title: $LL.ADD_CREDENTIALS.ADDRESS.TITLE(),
      description: $LL.ADD_CREDENTIALS.ADDRESS.DESCRIPTION(),
      icon: HouseFillIcon,
      link: '/me/add/address/info',
    },
  ];

  let creatingIdentity = false;

  const createEuropeanVerifiableIdentity = async () => {
    if ($appState.iota_wallet.did) {
      await goto('/me/iota-identity');
      return;
    }

    try {
      creatingIdentity = true;
      await dispatch({
        type: '[IOTA Wallet] Create or load',
        payload: { network: $appState.iota_wallet.network ?? 'testnet' },
      });
      await dispatch({ type: '[IOTA Wallet] Create identity', payload: {} });
    } finally {
      creatingIdentity = false;
    }
  };
</script>

<TopNavBar on:back={() => history.back()} title={$LL.ADD_CREDENTIALS.NAVBAR_TITLE()} class="sticky top-0 z-10" />

<div class="flex flex-col space-y-4 px-4 py-8">
  {#each data as { title, description, icon, link, disabled } (title)}
    <button
      class="flex w-full items-center justify-between rounded-xl bg-background-alt p-4 disabled:opacity-50"
      onclick={() => goto(link)}
      {disabled}
    >
      <div class="flex items-center space-x-4">
        <svelte:component this={icon} class="size-6 text-primary" />
        <div class="flex flex-col text-left">
          <p class="text-[14px]/[22px] font-medium text-slate-800 dark:text-grey">{title}</p>
          <p class="text-[12px]/[20px] font-medium text-slate-500 dark:text-slate-300">{description}</p>
        </div>
      </div>
      <CaretRightBoldIcon class="size-4 text-slate-500" />
    </button>
  {/each}

  <button
    class="flex w-full items-center justify-between rounded-xl bg-background-alt p-4 disabled:opacity-50"
    onclick={createEuropeanVerifiableIdentity}
    disabled={creatingIdentity}
  >
    <div class="flex items-center space-x-4">
      <SealCheckFillIcon class="size-6 text-primary" />
      <div class="flex flex-col text-left">
        <p class="text-[14px]/[22px] font-medium text-slate-800 dark:text-grey">European Verifiable Identity</p>
        <p class="text-[12px]/[20px] font-medium text-slate-500 dark:text-slate-300">
          {#if $appState.iota_wallet.did}
            Open your IOTA identity
          {:else if creatingIdentity}
            Creating identity
          {:else}
            Create an IOTA identity sponsored by gas stations
          {/if}
        </p>
      </div>
    </div>
    <CaretRightBoldIcon class="size-4 text-slate-500" />
  </button>

  {#if $appState.iota_wallet.last_error}
    <p class="rounded-xl bg-background-alt p-4 text-[12px]/[18px] font-medium text-rose-500">
      {$appState.iota_wallet.last_error}
    </p>
  {/if}
</div>
