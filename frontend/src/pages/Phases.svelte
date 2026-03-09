<script lang="ts">
  import { onMount } from 'svelte';
  import { phases, questions, options, inviteRules, groups as groupsApi, media } from '../lib/api';
  import type { Phase, Question, QuestionOption, Group, InviteRule, InviteRuleCondition, AvailableQuestion } from '../lib/types';
  import TelegramEditor from '../lib/TelegramEditor.svelte';
  import { toTelegramHtml } from '../lib/telegramify';
  import { t } from '../lib/i18n.svelte';

  let allPhases: Phase[] = $state([]);
  let selectedPhase: Phase | null = $state(null);
  let phaseQuestions: Question[] = $state([]);
  let questionOptionsMap: Record<number, QuestionOption[]> = $state({});
  let loading = $state(true);
  let error = $state('');
  let showAddForm = $state(false);

  // Normal phase forms
  let newPhaseName = $state('');
  let newPhaseDesc = $state('');
  let newPhaseType: 'normal' | 'invite' = $state('normal');
  let newQText = $state('');
  let newQType: Question['question_type'] = $state('text');
  let newOptLabel = $state('');
  let newOptValue = $state('');

  // Invite phase state
  let phaseInviteRules: InviteRule[] = $state([]);
  let allGroups: Group[] = $state([]);
  let availableQuestions: AvailableQuestion[] = $state([]);
  let selectedRuleId: number | null = $state(null);
  let ruleConditions: Record<number, InviteRuleCondition[]> = $state({});

  // Invite rule add form
  let showAddRuleForm = $state(false);
  let newRuleGroupId: number | null = $state(null);

  // Condition add form
  let addingConditionForRule: number | null = $state(null);
  let newCondQuestionId: number | null = $state(null);
  let newCondType = $state('');
  let newCondOptionId: number | null = $state(null);
  let newCondTextValue = $state('');

  // Info block form for invite phase
  let showAddInfoForm = $state(false);
  let newInfoText = $state('');

  // Media attachment state (shared for normal questions and info blocks)
  let newQMediaPath: string | null = $state(null);
  let newQMediaType: string | null = $state(null);
  let uploadingMedia = $state(false);
  let mediaError = $state('');
  // Info block media (invite phase)
  let newInfoMediaPath: string | null = $state(null);
  let newInfoMediaType: string | null = $state(null);
  let uploadingInfoMedia = $state(false);
  let infoMediaError = $state('');

  async function handleMediaUpload(file: File, target: 'question' | 'info') {
    const setUploading = target === 'question' ? (v: boolean) => uploadingMedia = v : (v: boolean) => uploadingInfoMedia = v;
    const setError = target === 'question' ? (v: string) => mediaError = v : (v: string) => infoMediaError = v;
    const setPath = target === 'question' ? (v: string | null) => newQMediaPath = v : (v: string | null) => newInfoMediaPath = v;
    const setType = target === 'question' ? (v: string | null) => newQMediaType = v : (v: string | null) => newInfoMediaType = v;

    setError('');
    setUploading(true);
    try {
      const result = await media.upload(file);
      setPath(result.media_path);
      setType(result.media_type);
    } catch (e: any) {
      setError(e.message || t('phases.uploadError'));
    } finally {
      setUploading(false);
    }
  }

  async function removeMedia(target: 'question' | 'info') {
    const path = target === 'question' ? newQMediaPath : newInfoMediaPath;
    if (path) {
      try { await media.delete(path); } catch {}
    }
    if (target === 'question') { newQMediaPath = null; newQMediaType = null; }
    else { newInfoMediaPath = null; newInfoMediaType = null; }
  }

  function resetMediaState(target: 'question' | 'info') {
    if (target === 'question') { newQMediaPath = null; newQMediaType = null; mediaError = ''; }
    else { newInfoMediaPath = null; newInfoMediaType = null; infoMediaError = ''; }
  }

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
    try {
      await phases.create({
        name: newPhaseName,
        description: newPhaseDesc || null,
        position: allPhases.length,
        phase_type: newPhaseType,
      });
      newPhaseName = ''; newPhaseDesc = ''; newPhaseType = 'normal';
      await load();
    } catch (e: any) {
      error = e.message;
    }
  }

  async function deletePhase(id: number) {
    if (!confirm(t('phases.deletePhaseConfirm'))) return;
    await phases.delete(id);
    if (selectedPhase?.id === id) { selectedPhase = null; phaseQuestions = []; questionOptionsMap = {}; phaseInviteRules = []; }
    await load();
  }

  async function toggleActive(phase: Phase) {
    try {
      await phases.update(phase.id, { ...phase, active: !phase.active });
      await load();
    } catch (e: any) {
      error = e.message;
    }
  }

  async function selectPhase(phase: Phase) {
    selectedPhase = phase;
    questionOptionsMap = {};
    showAddForm = false;
    showAddRuleForm = false;
    showAddInfoForm = false;
    selectedRuleId = null;
    addingConditionForRule = null;

    if (phase.phase_type === 'invite') {
      phaseQuestions = await questions.listByPhase(phase.id);
      phaseInviteRules = await inviteRules.listByPhase(phase.id);
      allGroups = await groupsApi.list();
      availableQuestions = await inviteRules.availableQuestions();
      // Load conditions for all rules
      const condMap: Record<number, InviteRuleCondition[]> = {};
      for (const rule of phaseInviteRules) {
        condMap[rule.id] = await inviteRules.listConditions(rule.id);
      }
      ruleConditions = condMap;
    } else {
      phaseQuestions = await questions.listByPhase(phase.id);
      phaseInviteRules = [];
      ruleConditions = {};
      await refreshAllOptions(phaseQuestions);
    }
  }

  // ── Normal phase question functions ──
  async function createQuestion() {
    if (!selectedPhase || !newQText.trim()) return;
    const telegramHtml = toTelegramHtml(newQText);
    await questions.create(selectedPhase.id, {
      text: telegramHtml,
      question_type: newQType,
      position: phaseQuestions.length,
      required: newQType !== 'info',
      media_path: newQMediaPath,
      media_type: newQMediaType as any,
    });
    newQText = '';
    resetMediaState('question');
    showAddForm = false;
    phaseQuestions = await questions.listByPhase(selectedPhase.id);
    await refreshAllOptions(phaseQuestions);
  }

  async function deleteQuestion(id: number) {
    if (!confirm(t('phases.deleteItemConfirm'))) return;
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

  function typeLabel(tp: Question['question_type']): string {
    const map: Record<string, string> = {
      text: t('phases.badgeText'),
      button: t('phases.badgeButton'),
      image: t('phases.badgeImage'),
      info: t('phases.badgeInfo'),
    };
    return map[tp] ?? tp;
  }

  let sortedPhases = $derived([...allPhases].sort((a, b) => a.position - b.position));
  let sortedQuestions = $derived([...phaseQuestions].sort((a, b) => a.position - b.position));

  async function movePhase(phase: Phase, dir: -1 | 1) {
    const idx = sortedPhases.findIndex(p => p.id === phase.id);
    const swapIdx = idx + dir;
    if (swapIdx < 0 || swapIdx >= sortedPhases.length) return;
    try {
      const other = sortedPhases[swapIdx];
      await phases.reorder([
        { id: phase.id, position: other.position },
        { id: other.id, position: phase.position },
      ]);
      await load();
    } catch (e: any) {
      error = e.message;
    }
  }

  function canMovePhase(phase: Phase, dir: -1 | 1): boolean {
    const idx = sortedPhases.findIndex(p => p.id === phase.id);
    const swapIdx = idx + dir;
    if (swapIdx < 0 || swapIdx >= sortedPhases.length) return false;
    const other = sortedPhases[swapIdx];
    // Prevent normal after invite
    if (dir === 1 && phase.phase_type === 'normal' && other.phase_type === 'invite') return false;
    if (dir === -1 && phase.phase_type === 'invite' && other.phase_type === 'normal') return false;
    return true;
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

  // ── Invite phase functions ──

  type PhaseItem =
    | { kind: 'info'; question: Question; position: number }
    | { kind: 'rule'; rule: InviteRule; position: number };

  let invitePhaseItems = $derived.by(() => {
    if (!selectedPhase || selectedPhase.phase_type !== 'invite') return [];
    const items: PhaseItem[] = [];
    for (const q of phaseQuestions) {
      if (q.question_type === 'info') {
        items.push({ kind: 'info', question: q, position: q.position });
      }
    }
    for (const r of phaseInviteRules) {
      items.push({ kind: 'rule', rule: r, position: r.position });
    }
    items.sort((a, b) => a.position - b.position);
    return items;
  });

  function getGroupTitle(groupId: number): string {
    return allGroups.find(g => g.id === groupId)?.title ?? `Group #${groupId}`;
  }

  function getQuestionLabel(questionId: number): string {
    const q = availableQuestions.find(aq => aq.id === questionId);
    if (!q) return `Question #${questionId}`;
    return `[${q.phase_name}] ${stripHtml(q.text).substring(0, 50)}`;
  }

  function stripHtml(html: string): string {
    const div = document.createElement('div');
    div.innerHTML = html;
    return div.textContent || div.innerText || '';
  }

  function getOptionLabel(questionId: number, optionId: number): string {
    const q = availableQuestions.find(aq => aq.id === questionId);
    if (!q) return `Option #${optionId}`;
    const opt = q.options.find(o => o.id === optionId);
    return opt?.label ?? `Option #${optionId}`;
  }

  function conditionTypeLabel(ct: string): string {
    const map: Record<string, string> = {
      option_selected: t('phases.wasSelected'),
      option_not_selected: t('phases.wasNotSelected'),
      text_contains: t('phases.contains'),
      text_not_contains: t('phases.doesNotContain'),
    };
    return map[ct] ?? ct;
  }

  let usedGroupIds = $derived(new Set(phaseInviteRules.map(r => r.group_id)));
  let availableGroupsForRule = $derived(allGroups.filter(g => !usedGroupIds.has(g.id)));

  async function createInviteRule() {
    if (!selectedPhase || !newRuleGroupId) return;
    const nextPos = invitePhaseItems.length > 0
      ? Math.max(...invitePhaseItems.map(i => i.position)) + 1
      : 0;
    await inviteRules.create(selectedPhase.id, { group_id: newRuleGroupId, position: nextPos });
    newRuleGroupId = null;
    showAddRuleForm = false;
    await selectPhase(selectedPhase);
  }

  async function deleteInviteRule(id: number) {
    if (!confirm(t('phases.deleteRuleConfirm'))) return;
    await inviteRules.delete(id);
    if (selectedRuleId === id) selectedRuleId = null;
    if (selectedPhase) await selectPhase(selectedPhase);
  }

  async function createInfoBlock() {
    if (!selectedPhase || !newInfoText.trim()) return;
    const telegramHtml = toTelegramHtml(newInfoText);
    const nextPos = invitePhaseItems.length > 0
      ? Math.max(...invitePhaseItems.map(i => i.position)) + 1
      : 0;
    await questions.create(selectedPhase.id, {
      text: telegramHtml,
      question_type: 'info',
      position: nextPos,
      required: false,
      media_path: newInfoMediaPath,
      media_type: newInfoMediaType as any,
    });
    newInfoText = '';
    resetMediaState('info');
    showAddInfoForm = false;
    await selectPhase(selectedPhase);
  }

  // Condition management
  let selectedCondQuestion = $derived(
    availableQuestions.find(q => q.id === newCondQuestionId) ?? null
  );

  function startAddCondition(ruleId: number) {
    addingConditionForRule = ruleId;
    newCondQuestionId = null;
    newCondType = '';
    newCondOptionId = null;
    newCondTextValue = '';
  }

  function cancelAddCondition() {
    addingConditionForRule = null;
  }

  async function saveCondition(ruleId: number) {
    if (!newCondQuestionId || !newCondType) return;
    await inviteRules.createCondition(ruleId, {
      question_id: newCondQuestionId,
      condition_type: newCondType,
      option_id: newCondType.startsWith('option_') ? newCondOptionId : null,
      text_value: newCondType.startsWith('text_') ? newCondTextValue : null,
    });
    addingConditionForRule = null;
    ruleConditions = {
      ...ruleConditions,
      [ruleId]: await inviteRules.listConditions(ruleId),
    };
  }

  async function deleteCondition(condId: number, ruleId: number) {
    await inviteRules.deleteCondition(condId);
    ruleConditions = {
      ...ruleConditions,
      [ruleId]: await inviteRules.listConditions(ruleId),
    };
  }
</script>

<div class="page-header">
  <h1>{t('phases.title')}</h1>
  <p class="page-desc">{t('phases.subtitle')}</p>
</div>

{#if error}<p class="error">{error}</p>{/if}

<div class="phases-layout">

  <!-- ── Phases sidebar ── -->
  <aside class="phases-sidebar">
    <div class="sidebar-header">
      <span class="sidebar-title">{t('phases.sidebar')}</span>
      {#if !loading}<span class="col-count">{allPhases.length}</span>{/if}
    </div>

    <div class="sidebar-list">
      {#if loading}
        <p class="state-msg">{t('common.loading')}</p>
      {:else if allPhases.length === 0}
        <p class="state-msg">{t('phases.none')}</p>
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
                {#if phase.phase_type === 'invite'}
                  <span class="badge badge-invite">{t('phases.menuInviteBadge')}</span>
                {:else if phase.active}
                  <span class="badge badge-active">{t('phases.menuActiveBadge')}</span>
                {:else}
                  <span class="badge badge-inactive">Off</span>
                {/if}
              </div>
              <div class="phase-actions">
                <button class="btn-icon" title={t('phases.moveUp')} disabled={!canMovePhase(phase, -1)} onclick={() => movePhase(phase, -1)}>&#8593;</button>
                <button class="btn-icon" title={t('phases.moveDown')} disabled={!canMovePhase(phase, 1)} onclick={() => movePhase(phase, 1)}>&#8595;</button>
                <button class="btn-sm" onclick={() => toggleActive(phase)}>
                  {phase.active ? t('common.disable') : t('common.enable')}
                </button>
                <button class="btn-sm danger" onclick={() => deletePhase(phase.id)}>{t('common.delete')}</button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="sidebar-add-form">
      <p class="add-form-label">{t('phases.newPhase')}</p>
      <form onsubmit={(e) => { e.preventDefault(); createPhase(); }}>
        <input bind:value={newPhaseName} placeholder={t('phases.phaseName')} required />
        <input bind:value={newPhaseDesc} placeholder={t('phases.descOptional')} />
        <select bind:value={newPhaseType} class="phase-type-select">
          <option value="normal">{t('phases.typeNormal')}</option>
          <option value="invite">{t('phases.typeInvite')}</option>
        </select>
        <button type="submit" class="btn-primary full-width">{t('phases.addPhase')}</button>
      </form>
    </div>
  </aside>

  <!-- ── Main area ── -->
  <main class="questions-main">

    {#if !selectedPhase}
      <div class="questions-empty-state">
        <div class="empty-icon">&#8592;</div>
        <p>{t('phases.selectPhase')}</p>
      </div>

    {:else if selectedPhase.phase_type === 'invite'}
      <!-- ══════ INVITE PHASE UI ══════ -->
      <div class="questions-header">
        <div class="questions-header-left">
          <h2 class="questions-phase-name">{selectedPhase.name}</h2>
          <span class="badge badge-invite">{t('phases.inviteBadge')}</span>
          <span class="col-count">{invitePhaseItems.length} {invitePhaseItems.length === 1 ? t('phases.items') : t('phases.itemsPlural')}</span>
        </div>
        <div class="header-buttons">
          <button class="btn-primary" onclick={() => { showAddInfoForm = !showAddInfoForm; showAddRuleForm = false; }}>
            {showAddInfoForm ? `${t('common.cancel')}` : t('phases.addInfoBlock')}
          </button>
          <button class="btn-primary btn-invite" onclick={() => { showAddRuleForm = !showAddRuleForm; showAddInfoForm = false; }}>
            {showAddRuleForm ? `${t('common.cancel')}` : t('phases.addInviteRule')}
          </button>
        </div>
      </div>

      {#if showAddInfoForm}
        <div class="add-question-form-card">
          <form onsubmit={(e) => { e.preventDefault(); createInfoBlock(); }}>
            <div class="form-field">
              <span class="field-label">{t('phases.infoBlockContent')}</span>
              <TelegramEditor
                content=""
                onchange={(html) => { newInfoText = html; }}
                placeholder={t('phases.infoBlockPlaceholder')}
              />
            </div>
            <div class="media-upload-section">
              {#if newInfoMediaPath}
                <div class="media-preview">
                  {#if newInfoMediaType === 'video'}
                    <!-- svelte-ignore a11y_media_has_caption -->
                    <video src="/{newInfoMediaPath}" controls class="media-thumb"></video>
                  {:else}
                    <img src="/{newInfoMediaPath}" alt="Attachment" class="media-thumb" />
                  {/if}
                  <button type="button" class="btn-sm danger" onclick={() => removeMedia('info')}>{t('phases.removeMedia')}</button>
                </div>
              {:else}
                <label class="media-upload-label">
                  {#if uploadingInfoMedia}
                    <span class="hint">{t('phases.uploading')}</span>
                  {:else}
                    <span class="media-upload-text">{t('phases.attachMedia')}</span>
                    <span class="hint">{t('phases.maxFileSize')}</span>
                  {/if}
                  <input type="file" accept="image/*,video/mp4,video/webm" class="media-file-input"
                    disabled={uploadingInfoMedia}
                    onchange={(e) => { const f = (e.target as HTMLInputElement).files?.[0]; if (f) handleMediaUpload(f, 'info'); (e.target as HTMLInputElement).value = ''; }} />
                </label>
                {#if infoMediaError}
                  <span class="error-text">{infoMediaError}</span>
                {/if}
              {/if}
            </div>
            <div class="form-bottom-row">
              <span class="hint">{t('phases.infoBlockDeliveryHint')}</span>
              <button type="submit" class="btn-primary" style="margin-left: auto">{t('phases.addInfoBlockBtn')}</button>
            </div>
          </form>
        </div>
      {/if}

      {#if showAddRuleForm}
        <div class="add-question-form-card">
          <form onsubmit={(e) => { e.preventDefault(); createInviteRule(); }}>
            <div class="form-field">
              <label class="field-label" for="new-rule-group">{t('phases.selectGroup')}</label>
              {#if availableGroupsForRule.length === 0}
                <p class="hint">{t('phases.allGroupsHaveRules')}</p>
              {:else}
                <select id="new-rule-group" bind:value={newRuleGroupId} required>
                  <option value={null} disabled selected>{t('phases.chooseGroup')}</option>
                  {#each availableGroupsForRule as g (g.id)}
                    <option value={g.id}>{g.title} ({g.telegram_id})</option>
                  {/each}
                </select>
              {/if}
            </div>
            <div class="form-bottom-row">
              <span class="hint">{t('phases.conditionsAfterCreate')}</span>
              <button type="submit" class="btn-primary" style="margin-left: auto" disabled={!newRuleGroupId}>{t('phases.addRuleBtn')}</button>
            </div>
          </form>
        </div>
      {/if}

      {#if invitePhaseItems.length === 0 && !showAddInfoForm && !showAddRuleForm}
        <p class="state-msg-centered">{t('phases.noItems')}</p>
      {:else}
        <div class="question-cards">
          {#each invitePhaseItems as item, idx (item.kind === 'info' ? `info-${item.question.id}` : `rule-${item.rule.id}`)}
            {#if item.kind === 'info'}
              <!-- Info block card -->
              <div class="question-card info-block">
                <div class="qcard-header">
                  <span class="pos-badge">#{idx + 1}</span>
                  <span class="type-badge type-info">{t('phases.badgeInfo')}</span>
                  <div class="qcard-actions">
                    <button class="btn-sm danger" onclick={() => deleteQuestion(item.question.id)}>{t('common.delete')}</button>
                  </div>
                </div>
                <div class="qcard-text">{@html item.question.text}</div>
                {#if item.question.media_path}
                  <div class="qcard-media">
                    {#if item.question.media_type === 'video'}
                      <!-- svelte-ignore a11y_media_has_caption -->
                      <video src="/{item.question.media_path}" controls class="qcard-media-thumb"></video>
                    {:else}
                      <img src="/{item.question.media_path}" alt="Attachment" class="qcard-media-thumb" />
                    {/if}
                    <button class="btn-sm danger" onclick={async () => {
                      try { await media.delete(item.question.media_path!); } catch {}
                      await questions.update(item.question.id, { ...item.question, media_path: null, media_type: null });
                      if (selectedPhase) await selectPhase(selectedPhase);
                    }}>{t('phases.removeMedia')}</button>
                  </div>
                {/if}
              </div>
            {:else}
              <!-- Invite rule card -->
              {@const rule = item.rule}
              {@const conditions = ruleConditions[rule.id] ?? []}
              <div class="question-card invite-rule-card">
                <div class="qcard-header">
                  <span class="pos-badge">#{idx + 1}</span>
                  <span class="type-badge type-invite">{t('phases.inviteRule')}</span>
                  <span class="rule-group-name">{getGroupTitle(rule.group_id)}</span>
                  <div class="qcard-actions">
                    <span class="col-count">{conditions.length} {conditions.length === 1 ? t('phases.conditions') : t('phases.conditionsPlural')}</span>
                    <button class="btn-sm danger" onclick={() => deleteInviteRule(rule.id)}>{t('common.delete')}</button>
                  </div>
                </div>

                <div class="rule-body">
                  {#if conditions.length === 0}
                    <p class="rule-always">{t('phases.alwaysSends')}</p>
                  {:else}
                    <div class="conditions-list">
                      {#each conditions as cond (cond.id)}
                        <div class="condition-row">
                          <span class="cond-question">{getQuestionLabel(cond.question_id)}</span>
                          {#if cond.condition_type.startsWith('option_')}
                            <span class="cond-operator">{conditionTypeLabel(cond.condition_type)}</span>
                            <span class="cond-value">{cond.option_id ? getOptionLabel(cond.question_id, cond.option_id) : '?'}</span>
                          {:else}
                            <span class="cond-operator">{conditionTypeLabel(cond.condition_type)}</span>
                            <span class="cond-value">"{cond.text_value ?? ''}"</span>
                          {/if}
                          <button class="chip-delete" title={t('phases.removeCondition')} onclick={() => deleteCondition(cond.id, rule.id)}>&#215;</button>
                        </div>
                      {/each}
                    </div>
                  {/if}

                  {#if addingConditionForRule === rule.id}
                    <div class="add-condition-form">
                      <div class="cond-form-row">
                        <select bind:value={newCondQuestionId} class="cond-select">
                          <option value={null} disabled selected>{t('phases.selectQuestion')}</option>
                          {#each availableQuestions as aq (aq.id)}
                            <option value={aq.id}>[{aq.phase_name}] {stripHtml(aq.text).substring(0, 60)}</option>
                          {/each}
                        </select>
                      </div>

                      {#if selectedCondQuestion}
                        <div class="cond-form-row">
                          {#if selectedCondQuestion.question_type === 'button'}
                            <select bind:value={newCondType} class="cond-select">
                              <option value="" disabled selected>{t('phases.conditionType')}</option>
                              <option value="option_selected">{t('phases.optionSelected')}</option>
                              <option value="option_not_selected">{t('phases.optionNotSelected')}</option>
                            </select>
                            {#if newCondType}
                              <select bind:value={newCondOptionId} class="cond-select">
                                <option value={null} disabled selected>{t('phases.selectOption')}</option>
                                {#each selectedCondQuestion.options as opt (opt.id)}
                                  <option value={opt.id}>{opt.label}</option>
                                {/each}
                              </select>
                            {/if}
                          {:else}
                            <select bind:value={newCondType} class="cond-select">
                              <option value="" disabled selected>{t('phases.conditionType')}</option>
                              <option value="text_contains">{t('phases.textContains')}</option>
                              <option value="text_not_contains">{t('phases.textNotContains')}</option>
                            </select>
                            {#if newCondType}
                              <input bind:value={newCondTextValue} placeholder={t('phases.searchText')} class="cond-input" />
                            {/if}
                          {/if}
                        </div>
                      {/if}

                      <div class="cond-form-actions">
                        <button class="btn-sm" onclick={cancelAddCondition}>{t('common.cancel')}</button>
                        <button class="btn-sm btn-confirm" onclick={() => saveCondition(rule.id)}
                          disabled={!newCondQuestionId || !newCondType || (newCondType.startsWith('option_') && !newCondOptionId) || (newCondType.startsWith('text_') && !newCondTextValue.trim())}>
                          {t('phases.saveCondition')}
                        </button>
                      </div>
                    </div>
                  {:else}
                    <button class="btn-sm btn-add-cond" onclick={() => startAddCondition(rule.id)}>{t('phases.addCondition')}</button>
                  {/if}
                </div>
              </div>
            {/if}
          {/each}
        </div>
      {/if}

    {:else}
      <!-- ══════ NORMAL PHASE UI ══════ -->
      <div class="questions-header">
        <div class="questions-header-left">
          <h2 class="questions-phase-name">{selectedPhase.name}</h2>
          <span class="col-count">{phaseQuestions.length} {phaseQuestions.length === 1 ? t('phases.questions') : t('phases.questionsPlural')}</span>
        </div>
        <button class="btn-primary" onclick={() => { showAddForm = !showAddForm; if (!showAddForm) { newQText = ''; newQType = 'text'; resetMediaState('question'); } }}>
          {showAddForm ? `${t('phases.cancelAdd')}` : t('phases.addQuestion')}
        </button>
      </div>

      {#if showAddForm}
        <div class="add-question-form-card">
          <form onsubmit={(e) => { e.preventDefault(); createQuestion(); }}>
            <div class="form-field">
              <label class="field-label" for="q-editor">{t('phases.content')}</label>
              <TelegramEditor
                content=""
                onchange={(html) => { newQText = html; }}
                placeholder={newQType === 'info' ? t('phases.infoBlockPlaceholder') : ''}
              />
            </div>
            <div class="media-upload-section">
              {#if newQMediaPath}
                <div class="media-preview">
                  {#if newQMediaType === 'video'}
                    <!-- svelte-ignore a11y_media_has_caption -->
                    <video src="/{newQMediaPath}" controls class="media-thumb"></video>
                  {:else}
                    <img src="/{newQMediaPath}" alt="Attachment" class="media-thumb" />
                  {/if}
                  <button type="button" class="btn-sm danger" onclick={() => removeMedia('question')}>{t('phases.removeMedia')}</button>
                </div>
              {:else}
                <label class="media-upload-label">
                  {#if uploadingMedia}
                    <span class="hint">{t('phases.uploading')}</span>
                  {:else}
                    <span class="media-upload-text">{t('phases.attachMedia')}</span>
                    <span class="hint">{t('phases.maxFileSize')}</span>
                  {/if}
                  <input type="file" accept="image/*,video/mp4,video/webm" class="media-file-input"
                    disabled={uploadingMedia}
                    onchange={(e) => { const f = (e.target as HTMLInputElement).files?.[0]; if (f) handleMediaUpload(f, 'question'); (e.target as HTMLInputElement).value = ''; }} />
                </label>
                {#if mediaError}
                  <span class="error-text">{mediaError}</span>
                {/if}
              {/if}
            </div>
            <div class="form-bottom-row">
              <select bind:value={newQType}>
                <option value="text">{t('phases.typeText')}</option>
                <option value="button">{t('phases.typeButton')}</option>
                <option value="image">{t('phases.typeImage')}</option>
                <option value="info">{t('phases.typeInfo')}</option>
              </select>
              {#if newQType === 'info'}
                <span class="hint">{t('phases.infoHint')}</span>
              {/if}
              <button type="submit" class="btn-primary" style="margin-left: auto">{t('phases.addQuestionBtn')}</button>
            </div>
          </form>
        </div>
      {/if}

      {#if phaseQuestions.length === 0 && !showAddForm}
        <p class="state-msg-centered">{t('phases.noQuestions')}</p>
      {:else}
        <div class="question-cards">
          {#each sortedQuestions as q, idx (q.id)}
            <div class="question-card" class:info-block={q.question_type === 'info'}>

              <div class="qcard-header">
                <span class="pos-badge">#{idx + 1}</span>

                {#if editingTypeId === q.id}
                  <select class="type-select" bind:value={editingTypeValue} onclick={(e) => e.stopPropagation()}>
                    <option value="text">{t('phases.badgeText')}</option>
                    <option value="button">{t('phases.badgeButton')}</option>
                    <option value="image">{t('phases.badgeImage')}</option>
                    <option value="info">{t('phases.badgeInfo')}</option>
                  </select>
                  <button class="btn-icon confirm" title="Save type" onclick={(e) => saveQuestionType(q, e)}>&#10003;</button>
                  <button class="btn-icon" title="Cancel" onclick={cancelEditType}>&#10005;</button>
                {:else}
                  <button class="type-badge type-{q.question_type}" title={t('phases.clickToChangeType')}
                    onclick={(e) => startEditType(q, e)}>
                    {typeLabel(q.question_type)}
                  </button>
                {/if}

                <div class="qcard-actions">
                  <button class="btn-icon" title="Move up" disabled={idx === 0} onclick={() => moveQuestion(q, -1)}>&#8593;</button>
                  <button class="btn-icon" title="Move down" disabled={idx === sortedQuestions.length - 1} onclick={() => moveQuestion(q, 1)}>&#8595;</button>
                  <button class="btn-sm danger" onclick={() => deleteQuestion(q.id)}>{t('common.delete')}</button>
                </div>
              </div>

              <div class="qcard-text">{@html q.text}</div>

              {#if q.media_path}
                <div class="qcard-media">
                  {#if q.media_type === 'video'}
                    <!-- svelte-ignore a11y_media_has_caption -->
                    <video src="/{q.media_path}" controls class="qcard-media-thumb"></video>
                  {:else}
                    <img src="/{q.media_path}" alt="Attachment" class="qcard-media-thumb" />
                  {/if}
                  <button class="btn-sm danger" onclick={async () => {
                    try { await media.delete(q.media_path!); } catch {}
                    await questions.update(q.id, { ...q, media_path: null, media_type: null });
                    if (selectedPhase) phaseQuestions = await questions.listByPhase(selectedPhase.id);
                  }}>{t('phases.removeMedia')}</button>
                </div>
              {/if}

              {#if q.question_type === 'button'}
                <div class="options-section">
                  <div class="options-header">
                    <span class="options-label">{t('phases.buttonOptions')}</span>
                    <span class="col-count">{questionOptionsMap[q.id]?.length ?? 0}</span>
                  </div>
                  <div class="options-chips">
                    {#each (questionOptionsMap[q.id] ?? []) as opt (opt.id)}
                      <span class="option-chip">
                        <span class="chip-label">{opt.label}</span>
                        {#if opt.value !== opt.label}
                          <span class="chip-value">{opt.value}</span>
                        {/if}
                        <button class="chip-delete" title={t('common.remove')} onclick={() => deleteOptionInline(opt.id, q.id)}>&#215;</button>
                      </span>
                    {/each}
                    {#if (questionOptionsMap[q.id] ?? []).length === 0}
                      <span class="options-empty">{t('phases.noOptions')}</span>
                    {/if}
                  </div>
                  <form class="add-option-row" onsubmit={(e) => { e.preventDefault(); createOptionFor(q.id); }}>
                    <input bind:value={newOptLabel} placeholder={t('phases.buttonLabel')} required />
                    <input bind:value={newOptValue} placeholder={t('phases.valueDefault')} />
                    <button type="submit" class="btn-sm">{t('phases.addOption')}</button>
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
  .sidebar-add-form input, .sidebar-add-form select {
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
  .sidebar-add-form input:focus, .sidebar-add-form select:focus {
    outline: none;
    border-color: #1a1a2e;
  }

  .phase-type-select {
    cursor: pointer;
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
  .badge-invite  { background: #eeeeff; color: #4444aa; }

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

  .header-buttons {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
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
  .add-question-form-card select {
    padding: 0.45rem 0.65rem;
    border: 1px solid #d0d3dc;
    border-radius: 4px;
    font-size: 0.875rem;
    background: white;
    color: #333;
    width: 100%;
    box-sizing: border-box;
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
  .type-invite  { background: #eeeeff; color: #4444aa; cursor: default; }

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
    align-items: center;
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

  /* ── Invite rule card ── */
  .invite-rule-card {
    border-left: 3px solid #9999dd;
    background: #f8f8ff;
  }
  .invite-rule-card .qcard-header {
    background: #f0f0fa;
  }

  .rule-group-name {
    font-size: 0.88rem;
    font-weight: 600;
    color: #1a1a2e;
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rule-body {
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .rule-always {
    font-size: 0.82rem;
    color: #888;
    font-style: italic;
    margin: 0;
  }

  .conditions-list {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .condition-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.5rem;
    background: #f0f0fa;
    border: 1px solid #ddddf0;
    border-radius: 4px;
    font-size: 0.8rem;
    flex-wrap: wrap;
  }

  .cond-question {
    color: #555;
    font-weight: 500;
    flex-shrink: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 300px;
  }

  .cond-operator {
    color: #4444aa;
    font-weight: 600;
    font-size: 0.75rem;
    text-transform: uppercase;
    flex-shrink: 0;
  }

  .cond-value {
    color: #1a7a3e;
    font-weight: 500;
    flex-shrink: 0;
  }

  /* ── Add condition form ── */
  .add-condition-form {
    border: 1px solid #ddddf0;
    border-radius: 6px;
    padding: 0.65rem;
    background: #f5f5ff;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .cond-form-row {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .cond-select {
    padding: 0.35rem 0.55rem;
    border: 1px solid #d0d3dc;
    border-radius: 4px;
    font-size: 0.8rem;
    background: white;
    color: #333;
    flex: 1;
    min-width: 120px;
  }

  .cond-input {
    padding: 0.35rem 0.55rem;
    border: 1px solid #d0d3dc;
    border-radius: 4px;
    font-size: 0.8rem;
    background: white;
    color: #333;
    flex: 1;
    min-width: 100px;
  }

  .cond-form-actions {
    display: flex;
    gap: 0.4rem;
    justify-content: flex-end;
  }

  .btn-add-cond {
    align-self: flex-start;
  }

  .btn-confirm {
    background: #e3f5eb;
    border-color: #a8d8bc;
    color: #1a7a3e;
  }
  .btn-confirm:hover:not(:disabled) {
    background: #d0eedb;
  }
  .btn-confirm:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .btn-invite {
    background: #4444aa;
  }
  .btn-invite:hover { opacity: 0.88; }

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

  /* ── Media upload ── */
  .media-upload-section {
    margin: 0.5rem 0;
  }
  .media-upload-label {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    border: 1px dashed #ccc;
    border-radius: 4px;
    cursor: pointer;
    background: #f9f9f9;
  }
  .media-upload-label:hover {
    border-color: #999;
    background: #f0f4ff;
  }
  .media-upload-text {
    font-size: 0.85rem;
    color: #333;
  }
  .media-file-input {
    display: none;
  }
  .media-preview {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
  }
  .media-thumb {
    max-width: 200px;
    max-height: 140px;
    border-radius: 4px;
    border: 1px solid #ddd;
    object-fit: cover;
  }
  .qcard-media {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    margin: 0.5rem 0;
  }
  .qcard-media-thumb {
    max-width: 120px;
    max-height: 90px;
    border-radius: 4px;
    border: 1px solid #ddd;
    object-fit: cover;
  }
  .error-text {
    color: #c0392b;
    font-size: 0.8rem;
    margin-top: 0.25rem;
  }
</style>
