<script lang="ts">
  import { TopNavBar } from '$lib/components';
  import { dispatch } from '$lib/dispatcher';
  import { InfoRegularIcon } from '$lib/icons';
  import { state } from '$lib/stores';

  $: preferred_key_type = $state.profile_settings.preferred_key_types.at(0);

  type KeyType = 'EdDSA' | 'ES256' | 'ES256K';

  interface Key {
    type: KeyType;
    alias?: string;
    key_id: string; // Corresponds to the `JwkStorage` in `did-manager`
    enabled: boolean;
  }

  const keys: Key[] = [
    {
      type: 'EdDSA',
      key_id: 'ed25519-0',
      enabled: true,
    },
    {
      type: 'ES256',
      key_id: 'es256-0',
      enabled: true,
    },
    // NOTE: Although Stronghold contains a key for ES256K, we're not giving the user that option yet since
    // the Rust library `jsonwebtoken` does not support it (yet) to sign tokens.
    // {
    //   type: 'ES256K',
    //   key_id: 'es256k-0',
    //   enabled: false,
    // },
  ];
</script>

<TopNavBar on:back={() => history.back()} title={'Manage keys'} class="sticky top-0 z-10" />

<div class="flex flex-col space-y-[15px] bg-silver px-4 py-5 dark:bg-navy">
  <div class="flex flex-col space-y-[10px]">
    <p class="text-[14px]/[22px] font-medium text-slate-500 dark:text-slate-300">IOTA testnet wallet</p>
    <div class="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-600 dark:bg-dark">
      <div class="flex items-center justify-between gap-3">
        <p class="text-base font-semibold text-slate-800 dark:text-grey">Transaction signing</p>
        <button
          class="rounded-lg bg-primary px-3 py-2 text-[12px]/[16px] font-semibold text-white dark:text-dark"
          on:click={() => dispatch({ type: '[IOTA Wallet] Create or load', payload: {} })}
        >
          Initialize
        </button>
      </div>
      <div class="mt-4 flex flex-col gap-2">
        <p class="text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">Address</p>
        <p class="font-mono text-[11px]/[16px] break-all text-slate-800 dark:text-grey">
          {$state.iota_wallet.address ?? 'Not initialized'}
        </p>
        {#if $state.iota_wallet.did}
          <p class="pt-2 text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">DID</p>
          <p class="font-mono text-[11px]/[16px] break-all text-slate-800 dark:text-grey">{$state.iota_wallet.did}</p>
        {/if}
        {#if $state.iota_wallet.identity_controller_cap}
          <p class="pt-2 text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">
            Identity controller cap
          </p>
          <p class="font-mono text-[11px]/[16px] break-all text-slate-800 dark:text-grey">
            {$state.iota_wallet.identity_controller_cap}
          </p>
        {/if}
        {#if $state.iota_wallet.last_transaction_digest}
          <p class="pt-2 text-[12px]/[18px] font-medium text-slate-500 dark:text-slate-300">Last tx</p>
          <p class="font-mono text-[11px]/[16px] break-all text-slate-800 dark:text-grey">
            {$state.iota_wallet.last_transaction_digest}
          </p>
        {/if}
        {#if $state.iota_wallet.last_error}
          <p class="pt-2 text-[12px]/[18px] font-medium text-rose-500">{$state.iota_wallet.last_error}</p>
        {/if}
      </div>
    </div>
  </div>

  <div class="flex flex-col space-y-[10px]">
    <p class="text-[14px]/[22px] font-medium text-slate-500 dark:text-slate-300">Available keys</p>
    {#each keys as key}
      <button
        class={`rounded-xl border bg-white p-4 disabled:opacity-30 dark:bg-dark ${key.type === preferred_key_type ? 'border-primary ring-1 ring-primary' : 'border-slate-200 dark:border-slate-600'}`}
        on:click={() => dispatch({ type: '[Keys] Set preferred key type', payload: { key_type: key.type } })}
        disabled={!key.enabled}
      >
        <div class="flex h-7 items-center justify-between">
          <div class="flex items-center">
            <p class="text-base font-semibold text-slate-800 dark:text-grey">{key.type}</p>
          </div>
          {#if key.type === preferred_key_type}
            <div class="flex items-center space-x-1 rounded-full bg-ex-blue-2 px-2 py-1 dark:bg-primary">
              <p class="text-[12px]/[20px] font-medium text-secondary dark:text-dark">preferred</p>
            </div>
          {/if}
        </div>
        {#if key.key_id}
          <div class="flex items-center justify-between space-x-4 pt-4">
            <p class="text-left font-mono text-[11px]/[14px] font-medium break-all text-slate-500 dark:text-slate-300">
              {key.key_id}
            </p>
          </div>
        {/if}
      </button>
    {/each}
  </div>

  <div class="flex w-full items-center rounded-lg bg-white px-4 py-4 dark:bg-dark">
    <span class="mr-4 h-6 w-6">
      <InfoRegularIcon class="h-6 w-6 text-primary" />
    </span>
    <div class="flex flex-col">
      <p class="text-[13px]/[24px] font-medium text-slate-800 dark:text-grey">Developer info</p>
      <ul class="ml-3 list-disc text-[12px]/[20px] font-medium text-slate-500 dark:text-slate-300">
        <li>All keys are generated once on profile creation.</li>
        <li>Only one key per type is currently supported.</li>
        <li>
          UniMe will automatically select the key type based on the server capabilities, but respect your preference if
          there's multiple matches.
        </li>
      </ul>
    </div>
  </div>
</div>
