<script lang="ts">
  import { onMount } from 'svelte';
  import { settings, debug } from '../lib/api';
  import { t, getLang, setLang, type Lang } from '../lib/i18n.svelte';

  // Language
  let selectedLang: Lang = $state(getLang());

  // LivePix settings
  let livepixUrl = $state('');
  let livepixPriceCents = $state('0');
  let livepixCurrency = $state('BRL');
  let loadingLivepix = $state(true);
  let savingLivepix = $state(false);
  let livepixMsg = $state('');

  // LivePix cached token (debug)
  let tokenValue = $state<string | null>(null);
  let tokenLoading = $state(false);
  let tokenFetched = $state(false);
  let tokenCopied = $state(false);

  let error = $state('');

  onMount(async () => {
    try {
      const [urlData, priceData, currencyData, langData] = await Promise.all([
        settings.get('livepix_account_url').catch(() => ({ value: '' })),
        settings.get('livepix_price_cents').catch(() => ({ value: '0' })),
        settings.get('livepix_currency').catch(() => ({ value: 'BRL' })),
        settings.get('language').catch(() => ({ value: 'en' })),
      ]);
      livepixUrl = urlData.value;
      livepixPriceCents = priceData.value;
      livepixCurrency = currencyData.value;
      selectedLang = langData.value as Lang;
    } catch (e: any) {
      error = e.message;
    } finally {
      loadingLivepix = false;
    }
  });

  async function saveLang() {
    try {
      await settings.update('language', selectedLang);
      setLang(selectedLang);
    } catch (e: any) {
      error = e.message;
    }
  }

  async function saveLivepix() {
    savingLivepix = true;
    livepixMsg = '';
    try {
      await Promise.all([
        settings.update('livepix_account_url', livepixUrl.trim()),
        settings.update('livepix_price_cents', livepixPriceCents.toString()),
        settings.update('livepix_currency', livepixCurrency.trim().toUpperCase()),
      ]);
      livepixMsg = t('common.saved');
      setTimeout(() => (livepixMsg = ''), 3000);
    } catch (e: any) {
      livepixMsg = `${t('common.error')}: ${e.message}`;
    } finally {
      savingLivepix = false;
    }
  }

  function priceDisplay(cents: string): string {
    const n = parseInt(cents, 10);
    if (isNaN(n)) return '0.00';
    return (n / 100).toFixed(2);
  }

  async function fetchToken() {
    tokenLoading = true;
    tokenFetched = false;
    try {
      const res = await debug.livepixToken();
      tokenValue = res.token;
      tokenFetched = true;
    } catch (e: any) {
      tokenValue = null;
      tokenFetched = true;
    } finally {
      tokenLoading = false;
    }
  }

  async function copyToken() {
    if (!tokenValue) return;
    await navigator.clipboard.writeText(tokenValue);
    tokenCopied = true;
    setTimeout(() => (tokenCopied = false), 2000);
  }

  function priceFromInput(value: string): string {
    const n = parseFloat(value);
    if (isNaN(n)) return '0';
    return Math.round(n * 100).toString();
  }
</script>

<h1>{t('settings.title')}</h1>

{#if error}<p class="error">{error}</p>{/if}

<section class="card">
  <h2>{t('settings.language')}</h2>
  <p class="hint">{t('settings.languageHint')}</p>
  <div class="field">
    <select bind:value={selectedLang} onchange={saveLang}>
      <option value="en">English</option>
      <option value="pt-BR">Português (Brasil)</option>
    </select>
  </div>
</section>

{#if loadingLivepix}
  <p>{t('common.loadingAlt')}</p>
{:else}
  <section class="card">
    <h2>{t('settings.livepix')}</h2>
    <p class="hint">
      {t('settings.livepixHint')}
    </p>

    <div class="field">
      <label for="lp-url">{t('settings.donationUrl')}</label>
      <input
        id="lp-url"
        type="url"
        placeholder="https://livepix.gg/youraccount"
        bind:value={livepixUrl}
      />
      <span class="hint">{t('settings.donationUrlHint')}</span>
    </div>

    <div class="field">
      <label for="lp-price">{t('settings.minPrice')} ({livepixCurrency})</label>
      <input
        id="lp-price"
        type="number"
        min="0"
        step="0.01"
        placeholder="0.00"
        value={priceDisplay(livepixPriceCents)}
        oninput={(e) => { livepixPriceCents = priceFromInput((e.target as HTMLInputElement).value); }}
      />
      <span class="hint">{t('settings.minPriceHint')} ({livepixPriceCents} {t('settings.cents')}).</span>
    </div>

    <div class="field">
      <label for="lp-currency">{t('settings.currencyCode')}</label>
      <input
        id="lp-currency"
        type="text"
        maxlength="10"
        placeholder="BRL"
        bind:value={livepixCurrency}
      />
      <span class="hint">{t('settings.currencyHint')}</span>
    </div>

    <div class="actions">
      <button onclick={saveLivepix} disabled={savingLivepix}>{savingLivepix ? t('common.saving') : t('common.save')}</button>
      {#if livepixMsg}
        <span class="save-msg" class:error-msg={livepixMsg.startsWith(t('common.error'))}>{livepixMsg}</span>
      {/if}
    </div>
  </section>
{/if}

<section class="card">
  <h2>{t('settings.livepixToken')}</h2>
  <p class="hint">
    {t('settings.tokenHint')}
  </p>
  <div class="actions">
    <button onclick={fetchToken} disabled={tokenLoading}>
      {tokenLoading ? t('settings.fetchingToken') : t('settings.viewToken')}
    </button>
    {#if tokenFetched && tokenValue}
      <button onclick={copyToken}>{tokenCopied ? t('settings.copied') : t('settings.copy')}</button>
    {/if}
  </div>
  {#if tokenFetched}
    {#if tokenValue}
      <textarea class="token-box" readonly>{tokenValue}</textarea>
    {:else}
      <p class="hint" style="margin-top:0.5rem">{t('settings.noToken')}</p>
    {/if}
  {/if}
</section>

<style>
  h1 { font-size: 1.4rem; margin-bottom: 1.5rem; color: #1a1a2e; }
  .card {
    background: white;
    border-radius: 8px;
    padding: 1.25rem 1.5rem;
    max-width: 720px;
    box-shadow: 0 1px 3px rgba(0,0,0,.07);
    margin-bottom: 1.5rem;
  }
  h2 { font-size: 1rem; border-bottom: 1px solid #ddd; padding-bottom: 0.5rem; margin: 0 0 0.75rem; }
  .hint { color: #666; font-size: 0.85rem; margin: 0 0 0.75rem; }
  .field { margin-bottom: 1rem; }
  .field label { display: block; font-size: 0.875rem; font-weight: 600; color: #333; margin-bottom: 0.3rem; }
  .field input, .field select {
    width: 100%;
    padding: 0.4rem 0.6rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    font-size: 0.9rem;
    box-sizing: border-box;
  }
  .field input:focus, .field select:focus { outline: none; border-color: #1a1a2e; }
  .field .hint { margin: 0.25rem 0 0; }
  .actions { display: flex; gap: 1rem; align-items: center; margin-top: 0.75rem; }
  button {
    padding: 0.4rem 1.1rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    background: #1a1a2e;
    color: white;
    font-size: 0.9rem;
  }
  button:disabled { opacity: 0.6; cursor: default; }
  .save-msg { font-size: 0.9rem; color: #27ae60; }
  .error-msg { color: #c0392b; }
  .error { color: #c0392b; }
  .token-box {
    width: 100%;
    margin-top: 0.75rem;
    padding: 0.5rem 0.6rem;
    font-family: monospace;
    font-size: 0.78rem;
    background: #f0f0f0;
    border: 1px solid #ccc;
    border-radius: 4px;
    resize: vertical;
    min-height: 5rem;
    box-sizing: border-box;
    color: #333;
    word-break: break-all;
  }
</style>
