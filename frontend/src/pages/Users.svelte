<script lang="ts">
  import { onMount } from 'svelte';
  import { users } from '../lib/api';
  import type { User, InviteLink } from '../lib/types';

  let allUsers: User[] = [];
  let loading = true;
  let error = '';
  let selected: User | null = null;
  let selectedLinks: InviteLink[] = [];
  let selectedAnswers: unknown[] = [];
  let actionMsg = '';

  onMount(async () => {
    try {
      allUsers = await users.list();
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  });

  async function selectUser(user: User) {
    selected = user;
    actionMsg = '';
    [selectedLinks, selectedAnswers] = await Promise.all([
      users.getInviteLinks(user.id),
      users.getAnswers(user.id),
    ]);
  }

  async function sendInvites(user: User) {
    try {
      await users.sendInvites(user.id);
      actionMsg = 'Invite links sent!';
    } catch (e: any) {
      actionMsg = `Error: ${e.message}`;
    }
  }

  async function revokeLinks(user: User) {
    if (!confirm('Revoke all unused invite links for this user?')) return;
    try {
      await users.revokeLinks(user.id);
      actionMsg = 'Links revoked.';
      selectedLinks = await users.getInviteLinks(user.id);
    } catch (e: any) {
      actionMsg = `Error: ${e.message}`;
    }
  }
</script>

<h1>Users</h1>

{#if error}<p class="error">{error}</p>{/if}

<div class="layout">
  <section>
    {#if loading}<p>Loading…</p>
    {:else if allUsers.length === 0}<p>No users yet.</p>
    {:else}
      <ul class="user-list">
        {#each allUsers as user (user.id)}
          <li>
            <button class:selected={selected?.id === user.id} on:click={() => selectUser(user)}>
              <strong>{user.first_name} {user.last_name ?? ''}</strong>
              <small>@{user.username ?? 'no username'} · id: {user.telegram_id}</small>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section>
    {#if (allUsers.length > 0 && !selected)}
      <p>Select a user to view details.</p>
    {:else if selected}
      <h2>{selected.first_name} {selected.last_name ?? ''}</h2>
      <p><strong>Telegram ID:</strong> {selected.telegram_id}</p>
      <p><strong>Username:</strong> @{selected.username ?? '—'}</p>
      <p><strong>Registered:</strong> {selected.created_at}</p>

      <div class="actions">
        <button on:click={() => sendInvites(selected!)}>Send Invite Links</button>
        <button class="warn" on:click={() => revokeLinks(selected!)}>Revoke Unused Links</button>
      </div>
      {#if actionMsg}<p class="msg">{actionMsg}</p>{/if}

      <h3>Invite Links ({selectedLinks.length})</h3>
      {#if selectedLinks.length === 0}<p>No links yet.</p>
      {:else}
        <ul class="links">
          {#each selectedLinks as link}
            <li class:used={!!link.used_at} class:revoked={!!link.revoked_at}>
              <code>{link.invite_link}</code>
              <small>
                {link.used_at ? `used ${link.used_at}` : link.revoked_at ? `revoked` : 'unused'}
              </small>
            </li>
          {/each}
        </ul>
      {/if}

      <h3>Answers ({selectedAnswers.length})</h3>
      {#if selectedAnswers.length === 0}<p>No answers yet.</p>
      {:else}
        <pre>{JSON.stringify(selectedAnswers, null, 2)}</pre>
      {/if}
    {/if}
  </section>
</div>

<style>
  .layout { display: grid; grid-template-columns: 300px 1fr; gap: 1.5rem; }
  .user-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.3rem; }
  .user-list li button { background: white; padding: 0.6rem 0.8rem; border-radius: 6px; cursor: pointer; display: flex; flex-direction: column; box-shadow: 0 1px 3px rgba(0,0,0,.07); width: 100%; text-align: left; font: inherit; color: inherit; }
  .user-list li button:hover { background: #f0f4ff; }
  .user-list li button.selected { border-left: 3px solid #1a1a2e; }
  small { color: #888; font-size: 0.75rem; }
  .actions { display: flex; gap: 0.5rem; margin: 0.75rem 0; }
  button { padding: 0.4rem 0.8rem; border: none; border-radius: 4px; cursor: pointer; background: #1a1a2e; color: white; }
  button.warn { background: #e67e22; }
  .msg { font-style: italic; color: #555; }
  .links { list-style: none; padding: 0; }
  .links li { padding: 0.3rem 0; display: flex; flex-direction: column; gap: 0.1rem; border-bottom: 1px solid #eee; }
  .links li.used code { color: #27ae60; }
  .links li.revoked { opacity: 0.5; }
  pre { background: #f5f5f5; padding: 0.75rem; border-radius: 4px; font-size: 0.8rem; overflow-x: auto; }
  .error { color: red; }
</style>
