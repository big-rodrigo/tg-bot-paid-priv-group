const BASE = '/api';

function authHeader(): Record<string, string> {
  // Read credentials from localStorage (set on first visit)
  const secret = localStorage.getItem('adminSecret') ?? '';
  const encoded = btoa(`admin:${secret}`);
  return { Authorization: `Basic ${encoded}`, 'Content-Type': 'application/json' };
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method,
    headers: authHeader(),
    body: body != null ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error ?? res.statusText);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

// ── Phases ────────────────────────────────────────────────────────────────
export const phases = {
  list: () => request<import('./types').Phase[]>('GET', '/phases'),
  create: (data: Partial<import('./types').Phase>) => request<import('./types').Phase>('POST', '/phases', data),
  update: (id: number, data: Partial<import('./types').Phase>) => request<import('./types').Phase>('PUT', `/phases/${id}`, data),
  delete: (id: number) => request<void>('DELETE', `/phases/${id}`),
  reorder: (items: { id: number; position: number }[]) => request<void>('PUT', '/phases/reorder', items),
};

// ── Questions ─────────────────────────────────────────────────────────────
export const questions = {
  listByPhase: (phaseId: number) => request<import('./types').Question[]>('GET', `/phases/${phaseId}/questions`),
  create: (phaseId: number, data: Partial<import('./types').Question>) =>
    request<import('./types').Question>('POST', `/phases/${phaseId}/questions`, data),
  update: (id: number, data: Partial<import('./types').Question>) =>
    request<import('./types').Question>('PUT', `/questions/${id}`, data),
  delete: (id: number) => request<void>('DELETE', `/questions/${id}`),
  reorder: (items: { id: number; position: number }[]) => request<void>('PUT', '/questions/reorder', items),
};

// ── Options ───────────────────────────────────────────────────────────────
export const options = {
  list: (questionId: number) =>
    request<import('./types').QuestionOption[]>('GET', `/questions/${questionId}/options`),
  create: (questionId: number, data: Partial<import('./types').QuestionOption>) =>
    request<import('./types').QuestionOption>('POST', `/questions/${questionId}/options`, data),
  update: (id: number, data: Partial<import('./types').QuestionOption>) =>
    request<void>('PUT', `/options/${id}`, data),
  delete: (id: number) => request<void>('DELETE', `/options/${id}`),
};

// ── Groups ────────────────────────────────────────────────────────────────
export const groups = {
  list: () => request<import('./types').Group[]>('GET', '/groups'),
  create: (data: { telegram_id: number; title: string }) =>
    request<import('./types').Group>('POST', '/groups', data),
  update: (id: number, data: Partial<import('./types').Group>) =>
    request<import('./types').Group>('PUT', `/groups/${id}`, data),
  delete: (id: number) => request<void>('DELETE', `/groups/${id}`),
};

// ── Users ─────────────────────────────────────────────────────────────────
export const users = {
  list: (page = 1, limit = 50) =>
    request<import('./types').User[]>('GET', `/users?page=${page}&limit=${limit}`),
  get: (id: number) => request<{ user: import('./types').User; registration: unknown }>('GET', `/users/${id}`),
  getAnswers: (id: number) => request<import('./types').EnrichedAnswer[]>('GET', `/users/${id}/answers`),
  getImageUrl: async (fileId: string): Promise<string> => {
    const res = await fetch(`${BASE}/admin/telegram-image?file_id=${encodeURIComponent(fileId)}`, {
      headers: authHeader(),
    });
    if (!res.ok) throw new Error('Failed to load image');
    const blob = await res.blob();
    return URL.createObjectURL(blob);
  },
  getInviteLinks: (id: number) => request<import('./types').InviteLink[]>('GET', `/users/${id}/invite_links`),
  sendInvites: (id: number) => request<void>('POST', `/admin/send-invites/${id}`),
  revokeLinks: (id: number) => request<unknown>('POST', `/admin/revoke-links/${id}`),
  resetRegistration: (id: number) => request<unknown>('POST', `/admin/reset-registration/${id}`),
  unregister: (id: number) => request<unknown>('POST', `/admin/unregister/${id}`),
};

// ── Invite Rules ─────────────────────────────────────────────────────────
export const inviteRules = {
  listByPhase: (phaseId: number) =>
    request<import('./types').InviteRule[]>('GET', `/phases/${phaseId}/invite-rules`),
  create: (phaseId: number, data: { group_id: number; position?: number }) =>
    request<import('./types').InviteRule>('POST', `/phases/${phaseId}/invite-rules`, data),
  update: (id: number, data: { group_id: number; position: number }) =>
    request<import('./types').InviteRule>('PUT', `/invite-rules/${id}`, data),
  delete: (id: number) => request<void>('DELETE', `/invite-rules/${id}`),
  reorder: (items: { id: number; position: number }[]) =>
    request<void>('PUT', '/invite-rules/reorder', items),
  listConditions: (ruleId: number) =>
    request<import('./types').InviteRuleCondition[]>('GET', `/invite-rules/${ruleId}/conditions`),
  createCondition: (ruleId: number, data: {
    question_id: number;
    condition_type: string;
    option_id?: number | null;
    text_value?: string | null;
  }) =>
    request<import('./types').InviteRuleCondition>('POST', `/invite-rules/${ruleId}/conditions`, data),
  deleteCondition: (id: number) => request<void>('DELETE', `/invite-rule-conditions/${id}`),
  availableQuestions: () =>
    request<import('./types').AvailableQuestion[]>('GET', '/invite-rules/questions'),
};

// ── Settings ──────────────────────────────────────────────────────────────
export const settings = {
  list: () => request<Record<string, string>>('GET', '/settings'),
  get: (key: string) => request<import('./types').Setting>('GET', `/settings/${key}`),
  update: (key: string, value: string) =>
    request<import('./types').Setting>('PUT', `/settings/${key}`, { value }),
};

// ── Payments ──────────────────────────────────────────────────────────────
export const payments = {
  list: (status?: string) =>
    request<import('./types').Payment[]>('GET', `/payments${status ? `?status=${status}` : ''}`),
};

// ── Media Upload ─────────────────────────────────────────────────────────
export const media = {
  upload: async (file: File): Promise<{ media_path: string; media_type: string }> => {
    const secret = localStorage.getItem('adminSecret') ?? '';
    const encoded = btoa(`admin:${secret}`);
    const form = new FormData();
    form.append('file', file);
    const res = await fetch(`${BASE}/upload`, {
      method: 'POST',
      headers: { Authorization: `Basic ${encoded}` },
      body: form,
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({ error: res.statusText }));
      throw new Error(err.error ?? res.statusText);
    }
    return res.json();
  },
  delete: async (mediaPath: string): Promise<void> => {
    await request<void>('DELETE', '/upload', { media_path: mediaPath });
  },
};

// ── Debug ─────────────────────────────────────────────────────────────────
export const debug = {
  livepixToken: () => request<{ token: string | null }>('GET', '/debug/livepix-token'),
};
