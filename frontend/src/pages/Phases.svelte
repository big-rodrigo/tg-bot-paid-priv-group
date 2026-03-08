<script lang="ts">
  import { onMount } from 'svelte';
  import { phases, questions, options } from '../lib/api';
  import type { Phase, Question, QuestionOption } from '../lib/types';
  import TelegramEditor from '../lib/TelegramEditor.svelte';
  import { toTelegramHtml } from '../lib/telegramify';

  let allPhases: Phase[] = $state([]);
  let selectedPhase: Phase | null = $state(null);
  let phaseQuestions: Question[] = $state([]);
  let questionOptionsMap: Record<number, QuestionOption[]> = $state({});
  let loading = $state(true);
  let error = $state('');
  let showAddForm = $state(false);

  // Forms
  let newPhaseName = $state('');
  let newPhaseDesc = $state('');
  let newQText = $state('');    // raw Tiptap HTML while composing
  let newQType: Question['question_type'] = $state('text');
  let newOptLabel = $state('');
  let newOptValue = $state('');

  async function load() {
    try {
      allPhases = await phases.list();
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  onMount(load);

  async function refreshAllOptions(qs: Question[]) {
    const buttonQs = qs.filter(q => q.question_type === 'button');
    const results = await Promise.all(buttonQs.map(q => options.list(q.id)));
    const map: Record<number, QuestionOption[]> = {};
    buttonQs.forEach((q, i) => { map[q.id] = results[i]; });
    questionOptionsMap = map;
  }

  async function createPhase() {
    if (!newPhaseName.trim()) return;
    await phases.create({ name: newPhaseName, description: newPhaseDesc || null, position: allPhases.length });
    newPhaseName = ''; newPhaseDesc = '';
    await load();
  }

  async function deletePhase(id: number) {
    if (!confirm('Delete this phase and all its questions?')) return;
    await phases.delete(id);
    if (selectedPhase?.id === id) { selectedPhase = null; phaseQuestions = []; questionOptionsMap = {}; }
    await load();
  }

  async function toggleActive(phase: Phase) {
    await phases.update(phase.id, { ...phase, active: !phase.active });
    await load();
  }

  async function selectPhase(phase: Phase) {
    selectedPhase = phase;
    questionOptionsMap = {};
    showAddForm = false;
    phaseQuestions = await questions.listByPhase(phase.id);
    await refreshAllOptions(phaseQuestions);
  }

  async function createQuestion() {
    if (!selectedPhase || !newQText.trim()) return;
    const telegramHtml = toTelegramHtml(newQText);
    await questions.create(selectedPhase.id, {
      text: telegramHtml,
      question_type: newQType,
      position: phaseQuestions.length,
      required: newQType !== 'info',
    });
    newQText = '';
    showAddForm = false;
    phaseQuestions = await questions.listByPhase(selectedPhase.id);
    await refreshAllOptions(phaseQuestions);
  }

  async function deleteQuestion(id: number) {
    if (!confirm('Delete this question?')) return;
    await questions.delete(id);
    const { [id]: _, ...rest } = questionOptionsMap;
    questionOptionsMap = rest;
    if (selectedPhase) phaseQuestions = await questions.listByPhase(selectedPhase.id);
  }

  async function createOptionFor(questionId: number) {
    if (!newOptLabel.trim()) return;
    await options.create(questionId, {
      label: newOptLabel,
      value: newOptValue || newOptLabel,
      position: (questionOptionsMap[questionId]?.length ?? 0),
    });
    newOptLabel = ''; newOptValue = '';
    questionOptionsMap = { ...questionOptionsMap, [questionId]: await options.list(questionId) };
  }

  async function deleteOptionInline(id: number, questionId: number) {
    await options.delete(id);
    questionOptionsMap = { ...questionOptionsMap, [questionId]: await options.list(questionId) };
  }

  function typeLabel(t: Question['question_type']): string {
    return { text: 'Text', button: 'Button', image: 'Image', info: 'Info Block' }[t] ?? t;
  }

  let sortedPhases = $derived([...allPhases].sort((a, b) => a.position - b.position));
  let sortedQuestions = $derived([...phaseQuestions].sort((a, b) => a.position - b.position));

  async function movePhase(phase: Phase, dir: -1 | 1) {
    const idx = sortedPhases.findIndex(p => p.id === phase.id);
    const swapIdx = idx + dir;
    if (swapIdx < 0 || swapIdx >= sortedPhases.length) return;
    const other = sortedPhases[swapIdx];
    await phases.reorder([
      { id: phase.id, position: other.position },
      { id: other.id, position: phase.position },
    ]);
    await load();
  }

  async function moveQuestion(q: Question, dir: -1 | 1) {
    const idx = sortedQuestions.findIndex(x => x.id === q.id);
    const swapIdx = idx + dir;
    if (swapIdx < 0 || swapIdx >= sortedQuestions.length) return;
    const other = sortedQuestions[swapIdx];
    await questions.reorder([
      { id: q.id, position: other.position },
      { id: other.id, position: q.position },
    ]);
    if (selectedPhase) phaseQuestions = await questions.listByPhase(selectedPhase.id);
  }

  let editingTypeId: number | null = $state(null);
  let editingTypeValue: Question['question_type'] = $state('text');

  function startEditType(q: Question, e: Event) {
    e.stopPropagation();
    editingTypeId = q.id;
    editingTypeValue = q.question_type;
  }

  function cancelEditType(e: Event) {
    e.stopPropagation();
    editingTypeId = null;
  }

  async function saveQuestionType(q: Question, e: Event) {
    e.stopPropagation();
    await questions.update(q.id, { ...q, question_type: editingTypeValue });
    editingTypeId = null;
    if (selectedPhase) phaseQuestions = await questions.listByPhase(selectedPhase.id);
    if (editingTypeValue === 'button') {
      questionOptionsMap = { ...questionOptionsMap, [q.id]: await options.list(q.id) };
    } else {
      const { [q.id]: _, ...rest } = questionOptionsMap;
      questionOptionsMap = rest;
    }
  }
</script>

<div class="page-header">
  <h1>Phases &amp; Questions</h1>
  <p class="page-desc">Build your registration flow by creating phases and adding questions to each one.</p>
</div>

{#if error}<p class="error">{error}</p>{/if}

<div class="phases-layout">

  <!-- ── Phases sidebar ── -->
  <aside class="phases-sidebar">
    <div class="sidebar-header">
      <span class="sidebar-title">Phases</span>
      {#if !loading}<span class="col-count">{allPhases.length}</span>{/if}
    </div>

    <div class="sidebar-list">
      {#if loading}
        <p class="state-msg">Loading…</p>
      {:else if allPhases.length === 0}
        <p class="state-msg">No phases yet.</p>
      {:else}
        <ul class="phase-list">
          {#each sortedPhases as phase, idx (phase.id)}
            <li class:selected={selectedPhase?.id === phase.id} class:inactive={!phase.active}>
              <div class="phase-item-body" role="button" tabindex="0"
                onclick={() => selectPhase(phase)}
                onkeydown={(e) => e.key === 'Enter' && selectPhase(phase)}>
                <span class="phase-pos">{idx + 1}</span>
                <div class="phase-text">
                  <strong class="phase-name">{phase.name}</strong>
                  {#if phase.description}<span class="phase-desc">{phase.description}</span>{/if}
                </div>
                {#if phase.active}
                  <span class="badge badge-active">Active</span>
                {:else}
                  <span class="badge badge-inactive">Off</span>
                {/if}
              </div>
              <div class="phase-actions">
                <button class="btn-icon" title="Move up" disabled={idx === 0} onclick={() => movePhase(phase, -1)}>↑</button>
                <button class="btn-icon" title="Move down" disabled={idx === sortedPhases.length - 1} onclick={() => movePhase(phase, 1)}>↓</button>
                <button class="btn-sm" onclick={() => toggleActive(phase)}>
                  {phase.active ? 'Disable' : 'Enable'}
                </button>
                <button class="btn-sm danger" onclick={() => deletePhase(phase.id)}>Delete</button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="sidebar-add-form">
      <p class="add-form-label">New phase</p>
      <form onsubmit={(e) => { e.preventDefault(); createPhase(); }}>
        <input bind:value={newPhaseName} placeholder="Phase name" required />
        <input bind:value={newPhaseDesc} placeholder="Description (optional)" />
        <button type="submit" class="btn-primary full-width">+ Add Phase</button>
      </form>
    </div>
  </aside>

  <!-- ── Questions main area ── -->
  <main class="questions-main">

    {#if !selectedPhase}
      <div class="questions-empty-state">
        <div class="empty-icon">←</div>
        <p>Select a phase from the sidebar to manage its questions</p>
      </div>

    {:else}
      <div class="questions-header">
        <div class="questions-header-left">
          <h2 class="questions-phase-name">{selectedPhase.name}</h2>
          <span class="col-count">{phaseQuestions.length} question{phaseQuestions.length === 1 ? '' : 's'}</span>
        </div>
        <button class="btn-primary" onclick={() => { showAddForm = !showAddForm; if (!showAddForm) { newQText = ''; newQType = 'text'; } }}>
          {showAddForm ? '✕ Cancel' : '+ Add Question'}
        </button>
      </div>

      {#if showAddForm}
        <div class="add-question-form-card">
          <form onsubmit={(e) => { e.preventDefault(); createQuestion(); }}>
            <div class="form-field">
              <label class="field-label" for="q-editor">Content</label>
              <TelegramEditor
                content=""
                onchange={(html) => { newQText = html; }}
                placeholder={newQType === 'info' ? 'Info block text…' : 'Question text…'}
              />
            </div>
            <div class="form-bottom-row">
              <select bind:value={newQType}>
                <option value="text">Text answer</option>
                <option value="button">Button choice</option>
                <option value="image">Image upload</option>
                <option value="info">Info block</option>
              </select>
              {#if newQType === 'info'}
                <span class="hint">Info blocks are sent to users without requiring a response.</span>
              {/if}
              <button type="submit" class="btn-primary" style="margin-left: auto">Add Question</button>
            </div>
          </form>
        </div>
      {/if}

      {#if phaseQuestions.length === 0 && !showAddForm}
        <p class="state-msg-centered">No questions yet. Click "Add Question" to start.</p>
      {:else}
        <div class="question-cards">
          {#each sortedQuestions as q, idx (q.id)}
            <div class="question-card" class:info-block={q.question_type === 'info'}>

              <div class="qcard-header">
                <span class="pos-badge">#{idx + 1}</span>

                {#if editingTypeId === q.id}
                  <select class="type-select" bind:value={editingTypeValue} onclick={(e) => e.stopPropagation()}>
                    <option value="text">Text</option>
                    <option value="button">Button</option>
                    <option value="image">Image</option>
                    <option value="info">Info Block</option>
                  </select>
                  <button class="btn-icon confirm" title="Save type" onclick={(e) => saveQuestionType(q, e)}>✓</button>
                  <button class="btn-icon" title="Cancel" onclick={cancelEditType}>✕</button>
                {:else}
                  <button class="type-badge type-{q.question_type}" title="Click to change type"
                    onclick={(e) => startEditType(q, e)}>
                    {typeLabel(q.question_type)}
                  </button>
                {/if}

                <div class="qcard-actions">
                  <button class="btn-icon" title="Move up" disabled={idx === 0} onclick={() => moveQuestion(q, -1)}>↑</button>
                  <button class="btn-icon" title="Move down" disabled={idx === sortedQuestions.length - 1} onclick={() => moveQuestion(q, 1)}>↓</button>
                  <button class="btn-sm danger" onclick={() => deleteQuestion(q.id)}>Delete</button>
                </div>
              </div>

              <div class="qcard-text">{@html q.text}</div>

              {#if q.question_type === 'button'}
                <div class="options-section">
                  <div class="options-header">
                    <span class="options-label">Button options</span>
                    <span class="col-count">{questionOptionsMap[q.id]?.length ?? 0}</span>
                  </div>
                  <div class="options-chips">
                    {#each (questionOptionsMap[q.id] ?? []) as opt (opt.id)}
                      <span class="option-chip">
                        <span class="chip-label">{opt.label}</span>
                        {#if opt.value !== opt.label}
                          <span class="chip-value">{opt.value}</span>
                        {/if}
                        <button class="chip-delete" title="Remove option" onclick={() => deleteOptionInline(opt.id, q.id)}>×</button>
                      </span>
                    {/each}
                    {#if (questionOptionsMap[q.id] ?? []).length === 0}
                      <span class="options-empty">No options yet</span>
                    {/if}
                  </div>
                  <form class="add-option-row" onsubmit={(e) => { e.preventDefault(); createOptionFor(q.id); }}>
                    <input bind:value={newOptLabel} placeholder="Button label" required />
                    <input bind:value={newOptValue} placeholder="Value (defaults to label)" />
                    <button type="submit" class="btn-sm">+ Add</button>
                  </form>
                </div>
              {/if}

            </div>
          {/each}
        </div>
      {/if}
    {/if}

  </main>

</div>

<style>
  /* ── Page header ── */
  .page-header {
    margin-bottom: 1.25rem;
  }
  h1 {
    font-size: 1.4rem;
    color: #1a1a2e;
    margin: 0 0 0.25rem 0;
  }
  .page-desc {
    font-size: 0.85rem;
    color: #888;
    margin: 0;
  }
  .error {
    color: #c0392b;
    margin-bottom: 1rem;
    font-size: 0.9rem;
  }

  /* ── Two-panel layout ── */
  .phases-layout {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: 1.25rem;
    align-items: start;
  }

  /* ── Phases sidebar ── */
  .phases-sidebar {
    display: flex;
    flex-direction: column;
    background: #ffffff;
    border: 1px solid #e2e4e9;
    border-radius: 8px;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.06);
    max-height: calc(100vh - 160px);
    overflow: hidden;
    position: sticky;
    top: 1rem;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #eee;
    background: #f9fafb;
    flex-shrink: 0;
  }
  .sidebar-title {
    font-size: 0.68rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: #1a1a2e;
    flex: 1;
  }

  .sidebar-list {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .sidebar-add-form {
    flex-shrink: 0;
    padding: 0.875rem 1rem;
    border-top: 1px solid #eee;
    background: #fcfcfd;
  }
  .add-form-label {
    font-size: 0.72rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #999;
    margin: 0 0 0.5rem;
  }
  .sidebar-add-form form {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .sidebar-add-form input {
    padding: 0.45rem 0.65rem;
    border: 1px solid #d0d3dc;
    border-radius: 4px;
    font-size: 0.875rem;
    width: 100%;
    box-sizing: border-box;
    background: white;
    color: #333;
    transition: border-color 0.15s;
  }
  .sidebar-add-form input:focus {
    outline: none;
    border-color: #1a1a2e;
  }

  /* ── Phase list ── */
  .phase-list {
    list-style: none;
    padding: 0.5rem;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .phase-list li {
    background: white;
    border: 1px solid #e8eaee;
    border-radius: 6px;
    overflow: hidden;
    transition: border-color 0.12s, box-shadow 0.12s;
  }
  .phase-list li:hover {
    border-color: #c8ccdf;
  }
  .phase-list li.selected {
    border-left: 3px solid #1a1a2e;
    background: #f0f4ff;
    border-color: #c0c8e8;
  }
  .phase-list li.inactive {
    opacity: 0.5;
  }

  .phase-item-body {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 0.75rem;
    cursor: pointer;
  }
  .phase-item-body:hover {
    background: rgba(0, 0, 0, 0.02);
  }

  .phase-pos {
    font-size: 0.7rem;
    font-weight: 700;
    color: #bbb;
    min-width: 1rem;
    flex-shrink: 0;
  }
  .phase-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .phase-name {
    font-size: 0.88rem;
    font-weight: 600;
    color: #1a1a2e;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .phase-desc {
    font-size: 0.75rem;
    color: #888;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .badge {
    font-size: 0.62rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0.1rem 0.4rem;
    border-radius: 10px;
    flex-shrink: 0;
  }
  .badge-active  { background: #e3f5eb; color: #1a7a3e; }
  .badge-inactive { background: #f0f0f0; color: #999; }

  .phase-actions {
    display: flex;
    gap: 0.3rem;
    align-items: center;
    padding: 0.3rem 0.75rem 0.5rem;
    border-top: 1px solid #f2f2f2;
    background: #fafafa;
  }

  /* ── Questions main area ── */
  .questions-main {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    min-width: 0;
  }

  .questions-empty-state {
    background: #ffffff;
    border: 1px solid #e2e4e9;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: 5rem 2rem;
    color: #ccc;
  }
  .empty-icon {
    font-size: 2rem;
    opacity: 0.4;
  }
  .questions-empty-state p {
    font-size: 0.9rem;
    color: #bbb;
    margin: 0;
    text-align: center;
  }

  .questions-header {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .questions-header-left {
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    min-width: 0;
  }
  .questions-phase-name {
    font-size: 1.1rem;
    font-weight: 700;
    color: #1a1a2e;
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Add-question form card ── */
  .add-question-form-card {
    background: #ffffff;
    border: 1px solid #e2e4e9;
    border-radius: 8px;
    padding: 1rem 1.25rem;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.06);
  }
  .add-question-form-card form {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .form-field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .field-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .form-bottom-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .form-bottom-row select {
    padding: 0.45rem 0.65rem;
    border: 1px solid #d0d3dc;
    border-radius: 4px;
    font-size: 0.875rem;
    background: white;
    color: #333;
    transition: border-color 0.15s;
  }
  .form-bottom-row select:focus {
    outline: none;
    border-color: #1a1a2e;
  }
  .hint {
    font-size: 0.78rem;
    color: #999;
    margin: 0;
    font-style: italic;
  }

  /* ── Question cards ── */
  .question-cards {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .question-card {
    background: #ffffff;
    border: 1px solid #e2e4e9;
    border-radius: 8px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
    overflow: hidden;
    transition: border-color 0.12s;
  }
  .question-card:hover {
    border-color: #c8ccdf;
  }
  .question-card.info-block {
    border-left: 3px solid #aad4f5;
    background: #f3f7ff;
  }

  .qcard-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.65rem 1rem;
    border-bottom: 1px solid #f0f0f0;
    background: #fafbfc;
  }
  .question-card.info-block .qcard-header {
    background: #eef4fb;
  }

  .pos-badge {
    font-size: 0.7rem;
    font-weight: 700;
    color: #bbb;
    min-width: 1.5rem;
    flex-shrink: 0;
  }

  .type-badge {
    font-size: 0.65rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0.15rem 0.55rem;
    border-radius: 10px;
    flex-shrink: 0;
    cursor: pointer;
    border: none;
    transition: opacity 0.15s;
  }
  .type-badge:hover { opacity: 0.75; }
  .type-text    { background: #e3f0fd; color: #1a6da8; }
  .type-button  { background: #e3f5eb; color: #1a7a3e; }
  .type-image   { background: #fdf3e3; color: #a86a1a; }
  .type-info    { background: #eeeeff; color: #4444aa; }

  .type-select {
    font-size: 0.72rem;
    padding: 0.15rem 0.35rem;
    border: 1px solid #d0d3dc;
    border-radius: 4px;
    background: white;
    color: #333;
    cursor: pointer;
  }

  .qcard-actions {
    display: flex;
    gap: 0.3rem;
    margin-left: auto;
    flex-shrink: 0;
  }

  .qcard-text {
    padding: 0.75rem 1rem;
    font-size: 0.9rem;
    color: #333;
    line-height: 1.5;
  }
  :global(.qcard-text b), :global(.qcard-text strong) { font-weight: 600; }
  :global(.qcard-text i), :global(.qcard-text em) { font-style: italic; }
  :global(.qcard-text code) { font-family: monospace; font-size: 0.85em; background: #f0f0f0; padding: 0 0.2em; border-radius: 2px; }
  :global(.qcard-text pre) { background: #f0f0f0; padding: 0.6em 0.8em; border-radius: 4px; overflow-x: auto; font-size: 0.85em; }
  :global(.qcard-text a) { color: #1a6da8; }

  /* ── Inline options section ── */
  .options-section {
    border-top: 1px solid #eef0f5;
    background: #f8faff;
    padding: 0.65rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .options-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .options-label {
    font-size: 0.68rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: #1a6da8;
  }

  .options-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    align-items: center;
  }

  .option-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    background: #e3f5eb;
    border: 1px solid #b8e0c8;
    border-radius: 14px;
    padding: 0.2rem 0.35rem 0.2rem 0.65rem;
    font-size: 0.8rem;
  }
  .chip-label {
    font-weight: 500;
    color: #1a7a3e;
  }
  .chip-value {
    font-size: 0.7rem;
    color: #888;
    font-family: monospace;
  }
  .chip-delete {
    background: transparent;
    border: none;
    cursor: pointer;
    color: #888;
    font-size: 1rem;
    line-height: 1;
    padding: 0 0.15rem;
    border-radius: 50%;
    transition: color 0.1s, background 0.1s;
  }
  .chip-delete:hover {
    color: #c0392b;
    background: rgba(192, 57, 43, 0.1);
  }

  .options-empty {
    font-size: 0.8rem;
    color: #bbb;
    font-style: italic;
  }

  .add-option-row {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    flex-wrap: wrap;
  }
  .add-option-row input {
    padding: 0.35rem 0.55rem;
    border: 1px solid #d0d3dc;
    border-radius: 4px;
    font-size: 0.8rem;
    background: white;
    color: #333;
    min-width: 0;
    transition: border-color 0.15s;
  }
  .add-option-row input:first-child { flex: 1.2; }
  .add-option-row input:nth-child(2) { flex: 1; }
  .add-option-row input:focus { outline: none; border-color: #1a1a2e; }

  /* ── Shared utilities ── */
  .state-msg {
    font-size: 0.85rem;
    color: #aaa;
    text-align: center;
    padding: 1.5rem 0;
    margin: 0;
  }
  .state-msg-centered {
    font-size: 0.88rem;
    color: #aaa;
    text-align: center;
    padding: 2rem 0;
    margin: 0;
  }
  .col-count {
    font-size: 0.7rem;
    font-weight: 600;
    background: #e8eaf0;
    color: #555;
    padding: 0.1rem 0.45rem;
    border-radius: 10px;
    flex-shrink: 0;
  }
  .full-width { width: 100%; }

  /* ── Buttons ── */
  .btn-primary {
    padding: 0.45rem 0.85rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    background: #1a1a2e;
    color: white;
    font-size: 0.85rem;
    font-weight: 500;
    white-space: nowrap;
    transition: opacity 0.15s;
  }
  .btn-primary:hover { opacity: 0.88; }

  .btn-sm {
    padding: 0.28rem 0.6rem;
    border: 1px solid #d0d3dc;
    border-radius: 4px;
    cursor: pointer;
    background: white;
    color: #555;
    font-size: 0.75rem;
    white-space: nowrap;
    transition: background 0.12s, border-color 0.12s;
    flex-shrink: 0;
  }
  .btn-sm:hover {
    background: #f0f0f0;
    border-color: #bbb;
  }
  .btn-sm.danger {
    border-color: #e8c0bc;
    color: #c0392b;
  }
  .btn-sm.danger:hover {
    background: #fdf0ef;
    border-color: #c0392b;
  }

  .btn-icon {
    padding: 0.2rem 0.35rem;
    border: 1px solid #d0d3dc;
    border-radius: 4px;
    cursor: pointer;
    background: white;
    color: #555;
    font-size: 0.8rem;
    line-height: 1.2;
    flex-shrink: 0;
    transition: background 0.12s, border-color 0.12s;
  }
  .btn-icon:hover:not(:disabled) {
    background: #f0f0f0;
    border-color: #bbb;
  }
  .btn-icon:disabled {
    opacity: 0.3;
    cursor: default;
  }
  .btn-icon.confirm {
    border-color: #a8d8bc;
    color: #1a7a3e;
  }
  .btn-icon.confirm:hover {
    background: #e3f5eb;
  }
</style>
