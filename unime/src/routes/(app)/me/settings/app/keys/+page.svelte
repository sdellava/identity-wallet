<script lang="ts">
  import { TopNavBar } from '$lib/components';
  import { dispatch } from '$lib/dispatcher';
  import { state } from '$lib/stores';

  type IotaNetwork = 'testnet' | 'mainnet';

  let selectedNetwork: IotaNetwork = $state.iota_wallet.network ?? 'testnet';
  let showSeed = false;

  $: if ($state.iota_wallet.network && $state.iota_wallet.network !== selectedNetwork) {
    selectedNetwork = $state.iota_wallet.network;
    showSeed = false;
  }

  const loadWallet = (network: IotaNetwork = selectedNetwork) =>
    dispatch({ type: '[IOTA Wallet] Create or load', payload: { network } });

  const changeNetwork = (network: IotaNetwork) => {
    selectedNetwork = network;
    showSeed = false;
    loadWallet(network);
  };
</script>

<TopNavBar on:back={() => history.back()} title={'IOTA wallet'} class="sticky top-0 z-10" />

<div class="flex flex-col gap-4 bg-silver px-4 py-5 dark:bg-navy">
  <section class="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-600 dark:bg-dark">
    <p class="text-base font-semibold text-slate-800 dark:text-grey">Network</p>
    <div class="mt-3 grid grid-cols-2 rounded-lg bg-silver p-1 dark:bg-navy">
      <button
        class="rounded-md px-3 py-2 text-[12px]/[16px] font-semibold {selectedNetwork === 'testnet'
          ? 'bg-primary text-white dark:text-dark'
          : 'text-slate-500 dark:text-slate-300'}"
        on:click={() => changeNetwork('testnet')}
      >
        Testnet
      </button>
      <button
        class="rounded-md px-3 py-2 text-[12px]/[16px] font-semibold {selectedNetwork === 'mainnet'
          ? 'bg-primary text-white dark:text-dark'
          : 'text-slate-500 dark:text-slate-300'}"
        on:click={() => changeNetwork('mainnet')}
      >
        Mainnet
      </button>
    </div>
  </section>

  <section class="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-600 dark:bg-dark">
    <div class="flex items-center justify-between gap-3">
      <div>
        <p class="text-base font-semibold text-slate-800 dark:text-grey">Seed and address</p>
        <p class="text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">
          Create or load the IOTA key stored in this wallet.
        </p>
      </div>
      <button
        class="rounded-lg bg-primary px-3 py-2 text-[12px]/[16px] font-semibold text-white dark:text-dark"
        on:click={() => loadWallet()}
      >
        {$state.iota_wallet.address ? 'Reload' : 'Create'}
      </button>
    </div>

    <div class="mt-4 flex flex-col gap-3">
      <div>
        <p class="text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">Address</p>
        <p class="font-mono text-[11px]/[16px] break-all text-slate-800 dark:text-grey">
          {$state.iota_wallet.address ?? 'Not created yet'}
        </p>
      </div>

      {#if $state.iota_wallet.public_key}
        <div>
          <p class="text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">Public key</p>
          <p class="font-mono text-[11px]/[16px] break-all text-slate-800 dark:text-grey">
            {$state.iota_wallet.public_key}
          </p>
        </div>
      {/if}

      {#if selectedNetwork === 'testnet' && $state.iota_wallet.seed_phrase}
        <div>
          <div class="flex items-center justify-between gap-3">
            <p class="text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">Seed phrase</p>
            <button
              class="rounded-lg border border-slate-200 px-3 py-1 text-[11px]/[16px] font-semibold text-slate-700 dark:border-slate-600 dark:text-grey"
              on:click={() => (showSeed = !showSeed)}
            >
              {showSeed ? 'Hide' : 'Show'}
            </button>
          </div>
          {#if showSeed}
            <p class="mt-2 font-mono text-[11px]/[16px] break-all text-slate-800 dark:text-grey">
              {$state.iota_wallet.seed_phrase}
            </p>
          {/if}
        </div>
      {/if}
    </div>
  </section>

  {#if $state.iota_wallet.did}
    <section class="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-600 dark:bg-dark">
      <div>
        <p class="text-base font-semibold text-slate-800 dark:text-grey">European Verifiable Identity</p>
        <p class="text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">
          Created on IOTA and controlled by this wallet address.
        </p>
      </div>
      <div class="mt-3">
        <p class="text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">DID</p>
        <p class="font-mono text-[11px]/[16px] break-all text-slate-800 dark:text-grey">{$state.iota_wallet.did}</p>
      </div>
    </section>
  {/if}

  {#if $state.iota_wallet.last_transaction_digest}
    <section class="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-600 dark:bg-dark">
      <p class="text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">Last signed transaction</p>
      <p class="font-mono text-[11px]/[16px] break-all text-slate-800 dark:text-grey">
        {$state.iota_wallet.last_transaction_digest}
      </p>
    </section>
  {/if}

  {#if $state.iota_wallet.last_error}
    <p class="rounded-xl bg-white p-4 text-[12px]/[18px] font-medium text-rose-500 dark:bg-dark">
      {$state.iota_wallet.last_error}
    </p>
  {/if}
</div>
