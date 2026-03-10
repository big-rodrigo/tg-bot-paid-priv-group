<script lang="ts">
  import { onMount } from 'svelte';
  import { users } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import type { User, InviteLink, EnrichedAnswer } from '../lib/types';

  let allUsers: User[] = $state([]);
  let loading = $state(true);
  let searching = $state(false);
  let error = $state('');
  let selected: User | null = $state(null);
  let selectedRegistration: any = $state(null);
  let selectedLinks: InviteLink[] = $state([]);
  let selectedAnswers: EnrichedAnswer[] = $state([]);
  let actionMsg = $state('');
  let actionType: 'success' | 'error' | '' = $state('');
  let imageOverlay: string | null = $state(null);
  let search = $state('');
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  async function loadUsers(q?: string) {
    searching = true;
    try {
      allUsers = await users.list(1, 50, q || undefined);
    } catch (e: any) {
      error = e.message;
    } finally {
      searching = false;
    }
  }

  function onSearchInput() {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => loadUsers(search), 300);
  }

  function clearSearch() {
    search = '';
    loadUsers();
  }

  onMount(async () => {
    try {
      allUsers = await users.list();
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  });

  interface PhaseGroup {
    phaseId: number;
    phaseName: string;
    items: EnrichedAnswer[];
  }

  let groupedAnswers: PhaseGroup[] = $derived.by(() => {
    const map = new Map<number, PhaseGroup>();
    for (const a of selectedAnswers) {
      if (!map.has(a.phase_id)) {
        map.set(a.phase_id, { phaseId: a.phase_id, phaseName: a.phase_name, items: [] });
      }
      map.get(a.phase_id)!.items.push(a);
    }
    return [...map.values()];
  });

  async function selectUser(user: User) {
    selected = user;
    actionMsg = '';
    actionType = '';
    const detail = await users.get(user.id);
    selectedRegistration = detail.registration;
    [selectedLinks, selectedAnswers] = await Promise.all([
      users.getInviteLinks(user.id),
      users.getAnswers(user.id),
    ]);
  }

  async function refreshSelected() {
    if (!selected) return;
    await selectUser(selected);
  }

  async function sendInvites(user: User) {
    try {
      await users.sendInvites(user.id);
      actionMsg = t('users.invitesSent');
      actionType = 'success';
      await refreshSelected();
    } catch (e: any) {
      actionMsg = `${t('common.error')}: ${e.message}`;
      actionType = 'error';
    }
  }

  async function revokeLinks(user: User) {
    if (!confirm(t('users.revokeConfirm'))) return;
    try {
      await users.revokeLinks(user.id);
      actionMsg = t('users.linksRevoked');
      actionType = 'success';
      await refreshSelected();
    } catch (e: any) {
      actionMsg = `${t('common.error')}: ${e.message}`;
      actionType = 'error';
    }
  }

  async function resetRegistration(user: User) {
    const msg = getLang() === 'pt-BR'
      ? `Reiniciar cadastro de ${user.first_name}?\n\nIsso irá excluir todas as respostas, cancelar pagamentos pendentes e permitir que o usuário envie /start para recomeçar.`
      : `Reset registration for ${user.first_name}?\n\nThis will delete all their answers, cancel pending payments, and allow them to /start the registration from scratch.`;
    if (!confirm(msg)) return;
    try {
      await users.resetRegistration(user.id);
      actionMsg = t('users.resetSuccess');
      actionType = 'success';
      await refreshSelected();
    } catch (e: any) {
      actionMsg = `${t('common.error')}: ${e.message}`;
      actionType = 'error';
    }
  }

  async function unregister(user: User) {
    const msg = getLang() === 'pt-BR'
      ? `Descadastrar completamente ${user.first_name}?\n\nIsso irá:\n- Excluir todas as respostas\n- Cancelar/excluir todos os pagamentos\n- Revogar e excluir todos os links de convite\n- Remover o progresso do cadastro\n\nEsta ação não pode ser desfeita.`
      : `Fully unregister ${user.first_name}?\n\nThis will:\n- Delete all answers\n- Cancel/delete all payments\n- Revoke and delete all invite links\n- Remove registration progress\n\nThis action cannot be undone.`;
    if (!confirm(msg)) return;
    try {
      await users.unregister(user.id);
      actionMsg = t('users.unregisterSuccess');
      actionType = 'success';
      await refreshSelected();
    } catch (e: any) {
      actionMsg = `${t('common.error')}: ${e.message}`;
      actionType = 'error';
    }
  }

  function registrationStatus(reg: any): string {
    if (!reg) return t('users.statusNotStarted');
    if (reg.completed_at) return t('users.statusCompleted');
    if (reg.current_phase_id) return t('users.statusInProgress');
    return t('users.statusStarted');
  }

  function formatDate(iso: string): string {
    try {
      const d = new Date(iso);
      return d.toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' })
        + ' ' + d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
    } catch {
      return iso;
    }
  }

  function typeLabel(tp: string): string {
    const map: Record<string, string> = {
      text: t('users.typeText'),
      button: t('users.typeButton'),
      image: t('users.typeImage'),
    };
    return map[tp] ?? tp;
  }

  // Need getLang for confirm dialogs
  import { getLang } from '../lib/i18n.svelte';
</script>

<h1>{t('users.title')}</h1>

{#if error}<p class="error">{error}</p>{/if}

<div class="layout" class:detail-open={!!selected}>
  <section class="user-list-section">
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
        placeholder={t('users.searchPlaceholder')}
        bind:value={search}
        oninput={onSearchInput}
      />
      {#if searching}
        <span class="search-spinner"></span>
      {:else if search}
        <button class="search-clear" onclick={clearSearch} aria-label="Clear search">×</button>
      {/if}
    </div>
    {#if loading}<p class="list-msg">{t('common.loading')}</p>
    {:else if allUsers.length === 0}
      <p class="list-msg">{search ? t('users.noResults') : t('users.none')}</p>
    {:else}
      <ul class="user-list">
        {#each allUsers as user (user.id)}
          <li>
            <button class:selected={selected?.id === user.id} onclick={() => selectUser(user)}>
              <strong>{user.first_name} {user.last_name ?? ''}</strong>
              <small>@{user.username ?? t('users.noUsername')} · id: {user.telegram_id}</small>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="user-detail-section">
    {#if selected}
      <button class="back-btn" onclick={() => { selected = null; }}>← {t('users.title')}</button>
    {/if}
    {#if (allUsers.length > 0 && !selected)}
      <p>{t('users.selectUser')}</p>
    {:else if selected}
      <h2>{selected.first_name} {selected.last_name ?? ''}</h2>
      <p><strong>{t('users.telegramId')}</strong> {selected.telegram_id}</p>
      <p><strong>{t('users.username')}</strong> @{selected.username ?? '—'}</p>
      <p><strong>{t('users.registered')}</strong> {selected.created_at}</p>
      <p><strong>{t('users.registrationStatus')}</strong>
        <span class="badge" class:badge-completed={selectedRegistration?.completed_at}
              class:badge-progress={selectedRegistration && !selectedRegistration.completed_at}
              class:badge-none={!selectedRegistration}>
          {registrationStatus(selectedRegistration)}
        </span>
      </p>

      <div class="actions-group">
        <div class="actions-row">
          <button onclick={() => sendInvites(selected!)}>{t('users.sendInvites')}</button>
          <button class="warn" onclick={() => revokeLinks(selected!)}>{t('users.revokeLinks')}</button>
        </div>
        <div class="actions-row">
          <button class="warn" onclick={() => resetRegistration(selected!)}>{t('users.resetRegistration')}</button>
          <button class="danger" onclick={() => unregister(selected!)}>{t('users.unregister')}</button>
        </div>
      </div>
      {#if actionMsg}
        <p class="msg" class:msg-success={actionType === 'success'} class:msg-error={actionType === 'error'}>{actionMsg}</p>
      {/if}

      <h3>{t('users.inviteLinks')} ({selectedLinks.length})</h3>
      {#if selectedLinks.length === 0}<p>{t('users.noLinks')}</p>
      {:else}
        <ul class="links">
          {#each selectedLinks as link}
            <li class:used={!!link.used_at} class:revoked={!!link.revoked_at}>
              <code>{link.invite_link}</code>
              <small>
                {link.used_at ? `${t('users.used')} ${link.used_at}` : link.revoked_at ? t('users.revoked') : t('users.unused')}
              </small>
            </li>
          {/each}
        </ul>
      {/if}

      <h3>{t('users.answers')} ({selectedAnswers.length})</h3>
      {#if selectedAnswers.length === 0}
        <p>{t('users.noAnswers')}</p>
      {:else}
        <div class="answers-timeline">
          {#each groupedAnswers as group (group.phaseId)}
            <div class="phase-group">
              <div class="phase-header">
                <span class="phase-marker"></span>
                <span class="phase-label">{group.phaseName}</span>
                <span class="phase-count">{group.items.length}</span>
              </div>

              <div class="phase-answers">
                {#each group.items as answer (answer.answer_id)}
                  <div class="answer-row">
                    <div class="answer-q">
                      <span class="type-tag type-{answer.question_type}">{typeLabel(answer.question_type)}</span>
                      <span class="q-text">{@html answer.question_text}</span>
                    </div>
                    <div class="answer-v">
                      {#if answer.question_type === 'text'}
                        <p class="text-answer">{answer.text_value ?? ''}</p>
                      {:else if answer.question_type === 'button'}
                        <span class="option-pill">{answer.option_label ?? `option #${answer.option_id}`}</span>
                      {:else if answer.question_type === 'image' && answer.image_file_id}
                        {#await users.getImageUrl(answer.image_file_id)}
                          <div class="img-placeholder">{t('users.loadingImage')}</div>
                        {:then blobUrl}
                          <button class="img-thumb-btn" onclick={() => { imageOverlay = blobUrl; }}>
                            <img src={blobUrl} alt="User upload" class="answer-img" />
                          </button>
                        {:catch}
                          <span class="img-error">{t('users.imageError')}</span>
                        {/await}
                      {/if}
                    </div>
                    <small class="answer-ts">{formatDate(answer.answered_at)}</small>
                  </div>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </section>
</div>

{#if imageOverlay}
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div class="overlay" role="dialog" onclick={() => { imageOverlay = null; }}
       onkeydown={(e) => e.key === 'Escape' && (imageOverlay = null)}>
    <img src={imageOverlay} alt="Full size" class="overlay-img" />
    <a href={imageOverlay} download="image" class="overlay-download" onclick={(e) => e.stopPropagation()}>{t('users.download')}</a>
  </div>
{/if}

<style>
  .layout { display: grid; grid-template-columns: 300px 1fr; gap: 1.5rem; }
  .back-btn { display: none; background: none; border: none; color: #1a1a2e; font-size: 0.9rem; font-weight: 600; cursor: pointer; padding: 0 0 0.75rem; }
  .back-btn:hover { text-decoration: underline; }

  @media (max-width: 640px) {
    .layout { grid-template-columns: 1fr; gap: 0; }
    .user-list-section { display: block; }
    .user-detail-section { display: block; }
    .layout.detail-open .user-list-section { display: none; }
    .back-btn { display: block; }
  }

  /* ── Search ── */
  .search-wrap {
    position: relative;
    display: flex;
    align-items: center;
    margin-bottom: 0.65rem;
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
    padding: 0.45rem 2.2rem 0.45rem 2.1rem;
    border: 1px solid #ddd;
    border-radius: 6px;
    font: inherit;
    font-size: 0.85rem;
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
  .list-msg { color: #888; font-size: 0.88rem; padding: 0.4rem 0; }

  .user-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.3rem; }
  .user-list li button { background: white; padding: 0.6rem 0.8rem; border-radius: 6px; cursor: pointer; display: flex; flex-direction: column; box-shadow: 0 1px 3px rgba(0,0,0,.07); width: 100%; text-align: left; font: inherit; color: inherit; border: 1px solid transparent; }
  .user-list li button:hover { background: #f0f4ff; }
  .user-list li button.selected { border-left: 3px solid #1a1a2e; }
  small { color: #888; font-size: 0.75rem; }
  .actions-group { display: flex; flex-direction: column; gap: 0.4rem; margin: 0.75rem 0; }
  .actions-row { display: flex; gap: 0.5rem; flex-wrap: wrap; }
  button { padding: 0.4rem 0.8rem; border: none; border-radius: 4px; cursor: pointer; background: #1a1a2e; color: white; }
  button.warn { background: #e67e22; }
  button.danger { background: #c0392b; }
  .badge { display: inline-block; padding: 0.15rem 0.5rem; border-radius: 10px; font-size: 0.75rem; font-weight: 500; }
  .badge-completed { background: #27ae60; color: white; }
  .badge-progress { background: #e67e22; color: white; }
  .badge-none { background: #7f8c8d; color: white; }
  .msg { font-style: italic; padding: 0.4rem 0.6rem; border-radius: 4px; font-size: 0.85rem; }
  .msg-success { color: #1e7e34; background: #e8f5e9; }
  .msg-error { color: #c0392b; background: #fdecea; }
  .links { list-style: none; padding: 0; }
  .links li { padding: 0.3rem 0; display: flex; flex-direction: column; gap: 0.1rem; border-bottom: 1px solid #eee; }
  .links li.used code { color: #27ae60; }
  .links li.revoked { opacity: 0.5; }
  .error { color: red; }

  /* ── Answers timeline ── */
  .answers-timeline { display: flex; flex-direction: column; gap: 1.25rem; }
  .phase-group { background: #ffffff; border: 1px solid #e2e4e9; border-radius: 8px; box-shadow: 0 1px 3px rgba(0,0,0,.05); overflow: hidden; }
  .phase-header { display: flex; align-items: center; gap: 0.5rem; padding: 0.6rem 1rem; background: #f9fafb; border-bottom: 1px solid #eee; }
  .phase-marker { width: 8px; height: 8px; border-radius: 50%; background: #1a1a2e; flex-shrink: 0; }
  .phase-label { font-size: 0.78rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: #1a1a2e; flex: 1; }
  .phase-count { font-size: 0.7rem; font-weight: 600; background: #e8eaf0; color: #555; padding: 0.1rem 0.45rem; border-radius: 10px; flex-shrink: 0; }
  .phase-answers { display: flex; flex-direction: column; }
  .answer-row { padding: 0.7rem 1rem; border-bottom: 1px solid #f2f3f5; display: flex; flex-direction: column; gap: 0.35rem; }
  .answer-row:last-child { border-bottom: none; }
  .answer-q { display: flex; align-items: baseline; gap: 0.5rem; }
  .q-text { font-size: 0.82rem; color: #666; line-height: 1.4; }
  :global(.q-text b), :global(.q-text strong) { font-weight: 600; }
  :global(.q-text i), :global(.q-text em) { font-style: italic; }
  :global(.q-text code) { font-family: monospace; font-size: 0.85em; background: #f0f0f0; padding: 0 0.2em; border-radius: 2px; }
  :global(.q-text a) { color: #1a6da8; }
  .type-tag { font-size: 0.6rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.1rem 0.45rem; border-radius: 10px; flex-shrink: 0; white-space: nowrap; }
  .type-text   { background: #e3f0fd; color: #1a6da8; }
  .type-button { background: #e3f5eb; color: #1a7a3e; }
  .type-image  { background: #fdf3e3; color: #a86a1a; }
  .answer-v { padding-left: 0.15rem; }
  .answer-v p { margin: 0; font-size: 0.92rem; color: #333; line-height: 1.5; }
  .option-pill { display: inline-block; font-size: 0.82rem; font-weight: 500; color: #1a7a3e; background: #e3f5eb; border: 1px solid #b8e0c8; padding: 0.15rem 0.6rem; border-radius: 14px; }
  .img-thumb-btn { background: none; border: 1px solid #e2e4e9; border-radius: 6px; padding: 2px; cursor: pointer; transition: border-color 0.15s; }
  .img-thumb-btn:hover { border-color: #1a1a2e; }
  .answer-img { display: block; max-width: 220px; max-height: 160px; border-radius: 4px; object-fit: cover; }
  .img-placeholder { font-size: 0.82rem; color: #999; font-style: italic; padding: 0.5rem 0; }
  .img-error { font-size: 0.82rem; color: #c0392b; }
  .answer-ts { font-size: 0.68rem; color: #aaa; display: block; }

  /* ── Image overlay ── */
  .overlay { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.8); display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 1rem; z-index: 1000; cursor: pointer; }
  .overlay-img { max-width: 90vw; max-height: 80vh; border-radius: 6px; box-shadow: 0 4px 24px rgba(0,0,0,.4); cursor: default; }
  .overlay-download { padding: 0.45rem 1.2rem; background: #ffffff; color: #1a1a2e; border-radius: 4px; font-size: 0.85rem; font-weight: 600; text-decoration: none; cursor: pointer; transition: opacity 0.15s; }
  .overlay-download:hover { opacity: 0.85; }
</style>
