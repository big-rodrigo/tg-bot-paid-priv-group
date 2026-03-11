<script lang="ts">
  import { onMount } from 'svelte';
  import { groups } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import type { Group } from '../lib/types';

  let allGroups: Group[] = [];
  let loading = true;
  let saving = false;
  let error = '';
  let newTelegramId = '';
  let newTitle = '';

  async function load() {
    try {
      allGroups = await groups.list();
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  onMount(load);

  async function create() {
    const telegram_id = parseInt(newTelegramId);
    if (!telegram_id || !newTitle.trim()) return;
    saving = true;
    error = '';
    try {
      await groups.create({ telegram_id, title: newTitle });
      newTelegramId = ''; newTitle = '';
      await load();
    } catch (e: any) {
      error = e.message;
    } finally {
      saving = false;
    }
  }

  async function toggle(group: Group) {
    saving = true;
    error = '';
    try {
      await groups.update(group.id, { ...group, active: !group.active });
      await load();
    } catch (e: any) {
      error = e.message;
    } finally {
      saving = false;
    }
  }

  async function remove(id: number) {
    if (!confirm(t('groups.removeConfirm'))) return;
    saving = true;
    error = '';
    try {
      await groups.delete(id);
      await load();
    } catch (e: any) {
      error = e.message;
    } finally {
      saving = false;
    }
  }
</script>

<h1>{t('groups.title')}</h1>
<p>{@html t('groups.subtitle')}</p>

{#if error}<p class="error">{error}</p>{/if}

<form on:submit|preventDefault={create} class="add-form">
  <input bind:value={newTelegramId} placeholder={t('groups.telegramIdPlaceholder')} required disabled={saving} />
  <input bind:value={newTitle} placeholder={t('groups.titlePlaceholder')} required disabled={saving} />
  <button type="submit" disabled={saving}>{saving ? '…' : t('groups.addGroup')}</button>
</form>

{#if loading}
  <p>{t('common.loadingAlt')}</p>
{:else if allGroups.length === 0}
  <p>{t('groups.none')}</p>
{:else}
  <div class="table-wrap">
    <table>
      <thead>
        <tr><th>{t('groups.id')}</th><th>{t('groups.telegramId')}</th><th>{t('groups.titleHeader')}</th><th>{t('groups.active')}</th><th>{t('common.actions')}</th></tr>
      </thead>
      <tbody>
        {#each allGroups as g (g.id)}
          <tr class:inactive={!g.active}>
            <td>{g.id}</td>
            <td><code>{g.telegram_id}</code></td>
            <td>{g.title}</td>
            <td>{g.active ? '✅' : '❌'}</td>
            <td>
              <button on:click={() => toggle(g)} disabled={saving}>{g.active ? t('common.disable') : t('common.enable')}</button>
              <button class="danger" on:click={() => remove(g.id)} disabled={saving}>{t('common.remove')}</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .add-form { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; flex-wrap: wrap; }
  .add-form input { flex: 1; min-width: 160px; padding: 0.4rem 0.6rem; border: 1px solid #ccc; border-radius: 4px; }
  button { padding: 0.4rem 0.8rem; border: none; border-radius: 4px; cursor: pointer; background: #1a1a2e; color: white; }
  button.danger { background: #c0392b; }
  button:disabled { opacity: 0.6; cursor: default; }
  .table-wrap { overflow-x: auto; -webkit-overflow-scrolling: touch; }
  table { width: 100%; border-collapse: collapse; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 1px 4px rgba(0,0,0,.1); }
  th, td { padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #eee; white-space: nowrap; }
  th { background: #f9f9f9; font-weight: 600; }
  tr.inactive { opacity: 0.55; }
  .error { color: red; }

  @media (max-width: 640px) {
    .add-form input { min-width: 0; }
    td, th { padding: 0.6rem 0.75rem; font-size: 0.85rem; }
  }
</style>
