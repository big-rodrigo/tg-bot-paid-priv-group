<script lang="ts">
  import { onMount } from 'svelte';
  import { payments } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import type { Payment } from '../lib/types';

  let allPayments: Payment[] = [];
  let loading = true;
  let error = '';
  let filter = '';
  let actionMsg = '';
  let completing: number | null = null;

  async function load() {
    loading = true;
    actionMsg = '';
    try {
      allPayments = await payments.list(filter || undefined);
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
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
  <label>
    {t('payments.filterByStatus')}
    <select bind:value={filter} on:change={load}>
      <option value="">{t('payments.all')}</option>
      <option value="pending">{t('payments.pending')}</option>
      <option value="completed">{t('payments.completed')}</option>
      <option value="failed">{t('payments.failed')}</option>
      <option value="refunded">{t('payments.refunded')}</option>
    </select>
  </label>
  <button on:click={load}>{t('common.refresh')}</button>
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
                  on:click={() => markComplete(p)}
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
  .filters { display: flex; align-items: center; gap: 1rem; margin-bottom: 1rem; flex-wrap: wrap; }
  select { padding: 0.35rem 0.5rem; border: 1px solid #ccc; border-radius: 4px; }
  button { padding: 0.35rem 0.75rem; border: none; border-radius: 4px; cursor: pointer; background: #1a1a2e; color: white; }
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
    td, th { padding: 0.6rem 0.75rem; font-size: 0.82rem; }
  }
</style>
