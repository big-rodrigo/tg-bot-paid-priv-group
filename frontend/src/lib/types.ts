export interface Phase {
  id: number;
  name: string;
  description: string | null;
  position: number;
  active: boolean;
}

export interface Question {
  id: number;
  phase_id: number;
  text: string;
  question_type: 'button' | 'text' | 'image' | 'info';
  position: number;
  required: boolean;
}

export interface Setting {
  key: string;
  value: string;
}

export interface QuestionOption {
  id: number;
  question_id: number;
  label: string;
  value: string;
  position: number;
}

export interface Group {
  id: number;
  telegram_id: number;
  title: string;
  active: boolean;
  created_at: string;
}

export interface User {
  id: number;
  telegram_id: number;
  username: string | null;
  first_name: string;
  last_name: string | null;
  created_at: string;
}

export interface Payment {
  id: number;
  user_id: number;
  provider: 'external' | 'telegram';
  external_ref: string | null;
  status: 'pending' | 'completed' | 'failed' | 'refunded';
  amount: number | null;
  currency: string | null;
  created_at: string;
}

export interface InviteLink {
  id: number;
  user_id: number;
  group_id: number;
  invite_link: string;
  created_at: string;
  used_at: string | null;
  revoked_at: string | null;
}
