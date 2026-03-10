<script lang="ts">
  import { onMount } from 'svelte';
  import { payments } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import type { Payment } from '../lib/types';

  let allPayments: Payment[] = $state([]);
  let loading = $state(true);
  let searching = $state(false);
  let error = $state('');
  let filter = $state('');
  let search = $state('');
  let actionMsg = $state('');
  let completing: number | null = $state(null);
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  async function load() {
    loading = true;
    actionMsg = '';
    error = '';
    try {
      allPayments = await payments.list(filter || undefined, search || undefined);
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
      searching = false;
    }
  }

  function onSearchInput() {
    searching = true;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(load, 300);
  }

  function clearSearch() {
    search = '';
    load();
  }

  onMount(load);

  function statusColor(status: Payment['status']) {
    return { pending: '#e67e22', completed: '#27ae60', failed: '#c0392b', refunded: '#7f8c8d' }[status];
  }

  async function markComplete(p: Payment) {
    if (!confirm(t('payments.markCompleteConfirm'))) return;
    completing = p.id;
    error = '';
    try {
      await payments.complete(p.id);
      actionMsg = t('payments.markCompleteSuccess');
      await load();
    } catch (e: any) {
      error = e.message;
    } finally {
      completing = null;
    }
  }
</script>

<h1>{t('payments.title')}</h1>

<div class="filters">
  <div class="search-wrap">
    <span class="search-icon">
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
        <circle cx="6.5" cy="6.5" r="5" stroke="#888" stroke-width="1.6"/>
        <line x1="10.35" y1="10.35" x2="14" y2="14" stroke="#888" stroke-width="1.6" stroke-linecap="round"/>
      </svg>
    </span>
    <input
      class="search-input"
      type="text"
      placeholder={t('payments.searchPlaceholder')}
      bind:value={search}
      oninput={onSearchInput}
    />
    {#if searching}
      <span class="search-spinner"></span>
    {:else if search}
      <button class="search-clear" onclick={clearSearch} aria-label="Clear search">×</button>
    {/if}
  </div>
  <label class="filter-label">
    {t('payments.filterByStatus')}
    <select bind:value={filter} onchange={load}>
      <option value="">{t('payments.all')}</option>
      <option value="pending">{t('payments.pending')}</option>
      <option value="completed">{t('payments.completed')}</option>
      <option value="failed">{t('payments.failed')}</option>
      <option value="refunded">{t('payments.refunded')}</option>
    </select>
  </label>
  <button onclick={load}>{t('common.refresh')}</button>
</div>

{#if actionMsg}<p class="success">{actionMsg}</p>{/if}
{#if error}<p class="error">{error}</p>{/if}

{#if loading}
  <p>{t('common.loadingAlt')}</p>
{:else if allPayments.length === 0}
  <p>{t('payments.none')}</p>
{:else}
  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th>{t('payments.id')}</th>
          <th>{t('payments.userId')}</th>
          <th>{t('payments.provider')}</th>
          <th>{t('payments.status')}</th>
          <th>{t('payments.amount')}</th>
          <th>{t('payments.reference')}</th>
          <th>{t('payments.created')}</th>
          <th>{t('payments.actions')}</th>
        </tr>
      </thead>
      <tbody>
        {#each allPayments as p (p.id)}
          <tr>
            <td>{p.id}</td>
            <td>{p.user_id}</td>
            <td>{p.provider}</td>
            <td><span class="badge" style="background:{statusColor(p.status)}">{p.status}</span></td>
            <td>{p.amount != null ? `${(p.amount / 100).toFixed(2)} ${p.currency ?? ''}` : '—'}</td>
            <td><code>{p.external_ref ?? '—'}</code></td>
            <td>{p.created_at}</td>
            <td>
              {#if p.status === 'pending'}
                <button
                  class="btn-complete"
                  disabled={completing === p.id}
                  onclick={() => markComplete(p)}
                >
                  {completing === p.id ? '…' : t('payments.markComplete')}
                </button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .filters {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }
  .filter-label { display: flex; align-items: center; gap: 0.4rem; font-size: 0.88rem; color: #555; white-space: nowrap; }
  select { padding: 0.38rem 0.5rem; border: 1px solid #ccc; border-radius: 4px; font: inherit; font-size: 0.88rem; }
  button { padding: 0.35rem 0.75rem; border: none; border-radius: 4px; cursor: pointer; background: #1a1a2e; color: white; font: inherit; }

  /* ── Search ── */
  .search-wrap {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 200px;
    max-width: 320px;
  }
  .search-icon {
    position: absolute;
    left: 0.65rem;
    display: flex;
    align-items: center;
    pointer-events: none;
  }
  .search-input {
    width: 100%;
    padding: 0.42rem 2.2rem 0.42rem 2.1rem;
    border: 1px solid #ddd;
    border-radius: 6px;
    font: inherit;
    font-size: 0.88rem;
    color: #333;
    background: #fff;
    box-sizing: border-box;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .search-input:focus {
    outline: none;
    border-color: #1a1a2e;
    box-shadow: 0 0 0 3px rgba(26,26,46,.08);
  }
  .search-input::placeholder { color: #aaa; }
  .search-clear {
    position: absolute;
    right: 0.5rem;
    background: none;
    border: none;
    color: #999;
    font-size: 1.1rem;
    line-height: 1;
    padding: 0.2rem 0.3rem;
    cursor: pointer;
    border-radius: 3px;
  }
  .search-clear:hover { color: #333; background: #f0f0f0; }
  .search-spinner {
    position: absolute;
    right: 0.65rem;
    width: 14px;
    height: 14px;
    border: 2px solid #ddd;
    border-top-color: #1a1a2e;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .table-wrap { overflow-x: auto; -webkit-overflow-scrolling: touch; }
  table { width: 100%; border-collapse: collapse; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 1px 4px rgba(0,0,0,.1); }
  th, td { padding: 0.7rem 1rem; text-align: left; border-bottom: 1px solid #eee; font-size: 0.9rem; white-space: nowrap; }
  th { background: #f9f9f9; font-weight: 600; }
  .badge { color: white; padding: 0.2rem 0.5rem; border-radius: 10px; font-size: 0.8rem; }
  .btn-complete { background: #27ae60; font-size: 0.8rem; padding: 0.25rem 0.6rem; }
  .btn-complete:disabled { opacity: 0.6; cursor: default; }
  .error { color: #c0392b; }
  .success { color: #27ae60; }

  @media (max-width: 640px) {
    .search-wrap { max-width: 100%; min-width: 0; flex: 1 1 100%; }
    td, th { padding: 0.6rem 0.75rem; font-size: 0.82rem; }
  }
</style>
