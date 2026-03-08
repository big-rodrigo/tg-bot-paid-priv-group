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
  getAnswers: (id: number) => request<unknown[]>('GET', `/users/${id}/answers`),
  getInviteLinks: (id: number) => request<import('./types').InviteLink[]>('GET', `/users/${id}/invite_links`),
  sendInvites: (id: number) => request<void>('POST', `/admin/send-invites/${id}`),
  revokeLinks: (id: number) => request<unknown>('POST', `/admin/revoke-links/${id}`),
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

// ── Debug ─────────────────────────────────────────────────────────────────
export const debug = {
  livepixToken: () => request<{ token: string | null }>('GET', '/debug/livepix-token'),
};
