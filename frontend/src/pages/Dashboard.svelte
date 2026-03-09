<script lang="ts">
  import { onMount } from 'svelte';
  import { users, phases, groups, payments } from '../lib/api';
  import { t } from '../lib/i18n.svelte';

  let stats = { users: 0, phases: 0, groups: 0, pending_payments: 0 };
  let loading = true;
  let error = '';

  onMount(async () => {
    try {
      const [u, p, g, pay] = await Promise.all([
        users.list(1, 1000),
        phases.list(),
        groups.list(),
        payments.list('pending'),
      ]);
      stats = {
        users: u.length,
        phases: p.length,
        groups: g.length,
        pending_payments: pay.length,
      };
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  });
</script>

<h1>{t('dash.title')}</h1>

{#if loading}
  <p>{t('common.loading')}</p>
{:else if error}
  <p class="error">{t('common.error')}: {error}</p>
{:else}
  <div class="stats">
    <div class="card"><span class="num">{stats.users}</span><span class="label">{t('dash.users')}</span></div>
    <div class="card"><span class="num">{stats.phases}</span><span class="label">{t('dash.phases')}</span></div>
    <div class="card"><span class="num">{stats.groups}</span><span class="label">{t('dash.groups')}</span></div>
    <div class="card warn"><span class="num">{stats.pending_payments}</span><span class="label">{t('dash.pendingPayments')}</span></div>
  </div>

  <p>{t('dash.hint')}</p>
{/if}

<style>
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 1rem;
    margin: 1.5rem 0;
  }
  .card {
    background: white;
    border-radius: 8px;
    padding: 1.25rem;
    text-align: center;
    box-shadow: 0 1px 4px rgba(0,0,0,.1);
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .card.warn { border-top: 3px solid #f0a500; }
  .num { font-size: 2rem; font-weight: 700; color: #1a1a2e; }
  .label { font-size: 0.85rem; color: #666; }
  .error { color: red; }
</style>
