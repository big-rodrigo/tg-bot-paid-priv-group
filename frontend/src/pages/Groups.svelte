<script lang="ts">
  import { onMount } from 'svelte';
  import { groups } from '../lib/api';
  import type { Group } from '../lib/types';

  let allGroups: Group[] = [];
  let loading = true;
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
    await groups.create({ telegram_id, title: newTitle });
    newTelegramId = ''; newTitle = '';
    await load();
  }

  async function toggle(group: Group) {
    await groups.update(group.id, { ...group, active: !group.active });
    await load();
  }

  async function remove(id: number) {
    if (!confirm('Remove this group?')) return;
    await groups.delete(id);
    await load();
  }
</script>

<h1>Groups</h1>
<p>Add the groups this bot manages. The bot must be an <strong>administrator</strong> of each group with permission to create invite links.</p>

{#if error}<p class="error">{error}</p>{/if}

<form on:submit|preventDefault={create} class="add-form">
  <input bind:value={newTelegramId} placeholder="Telegram group ID (e.g. -1001234567890)" required />
  <input bind:value={newTitle} placeholder="Group title" required />
  <button type="submit">Add Group</button>
</form>

{#if loading}
  <p>Loading…</p>
{:else if allGroups.length === 0}
  <p>No groups configured yet.</p>
{:else}
  <table>
    <thead>
      <tr><th>ID</th><th>Telegram ID</th><th>Title</th><th>Active</th><th>Actions</th></tr>
    </thead>
    <tbody>
      {#each allGroups as g (g.id)}
        <tr class:inactive={!g.active}>
          <td>{g.id}</td>
          <td><code>{g.telegram_id}</code></td>
          <td>{g.title}</td>
          <td>{g.active ? '✅' : '❌'}</td>
          <td>
            <button on:click={() => toggle(g)}>{g.active ? 'Disable' : 'Enable'}</button>
            <button class="danger" on:click={() => remove(g.id)}>Remove</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .add-form { display: flex; gap: 0.5rem; margin-bottom: 1.5rem; flex-wrap: wrap; }
  .add-form input { flex: 1; min-width: 200px; padding: 0.4rem 0.6rem; border: 1px solid #ccc; border-radius: 4px; }
  button { padding: 0.4rem 0.8rem; border: none; border-radius: 4px; cursor: pointer; background: #1a1a2e; color: white; }
  button.danger { background: #c0392b; }
  table { width: 100%; border-collapse: collapse; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 1px 4px rgba(0,0,0,.1); }
  th, td { padding: 0.75rem 1rem; text-align: left; border-bottom: 1px solid #eee; }
  th { background: #f9f9f9; font-weight: 600; }
  tr.inactive { opacity: 0.55; }
  .error { color: red; }
</style>
