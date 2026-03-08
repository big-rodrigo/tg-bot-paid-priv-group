<script lang="ts">
  import { onMount } from 'svelte';
  import { payments } from '../lib/api';
  import type { Payment } from '../lib/types';

  let allPayments: Payment[] = [];
  let loading = true;
  let error = '';
  let filter = '';

  async function load() {
    loading = true;
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
</script>

<h1>Payments</h1>

<div class="filters">
  <label>
    Filter by status:
    <select bind:value={filter} on:change={load}>
      <option value="">All</option>
      <option value="pending">Pending</option>
      <option value="completed">Completed</option>
      <option value="failed">Failed</option>
      <option value="refunded">Refunded</option>
    </select>
  </label>
  <button on:click={load}>Refresh</button>
</div>

{#if error}<p class="error">{error}</p>{/if}

{#if loading}
  <p>Loading…</p>
{:else if allPayments.length === 0}
  <p>No payments found.</p>
{:else}
  <table>
    <thead>
      <tr>
        <th>ID</th>
        <th>User ID</th>
        <th>Provider</th>
        <th>Status</th>
        <th>Amount</th>
        <th>Reference</th>
        <th>Created</th>
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
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .filters { display: flex; align-items: center; gap: 1rem; margin-bottom: 1rem; }
  select { padding: 0.35rem 0.5rem; border: 1px solid #ccc; border-radius: 4px; }
  button { padding: 0.35rem 0.75rem; border: none; border-radius: 4px; cursor: pointer; background: #1a1a2e; color: white; }
  table { width: 100%; border-collapse: collapse; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 1px 4px rgba(0,0,0,.1); }
  th, td { padding: 0.7rem 1rem; text-align: left; border-bottom: 1px solid #eee; font-size: 0.9rem; }
  th { background: #f9f9f9; font-weight: 600; }
  .badge { color: white; padding: 0.2rem 0.5rem; border-radius: 10px; font-size: 0.8rem; }
  .error { color: red; }
</style>
