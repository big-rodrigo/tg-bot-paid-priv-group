export interface Phase {
  id: number;
  name: string;
  description: string | null;
  position: number;
  active: boolean;
  phase_type: 'normal' | 'invite';
}

export interface Question {
  id: number;
  phase_id: number;
  text: string;
  question_type: 'button' | 'text' | 'image' | 'info';
  position: number;
  required: boolean;
  media_path: string | null;
  media_type: 'image' | 'video' | 'animation' | null;
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

export interface EnrichedAnswer {
  answer_id: number;
  text_value: string | null;
  option_id: number | null;
  image_file_id: string | null;
  answered_at: string;
  question_id: number;
  question_text: string;
  question_type: 'text' | 'button' | 'image';
  question_position: number;
  phase_id: number;
  phase_name: string;
  phase_position: number;
  option_label: string | null;
}

export interface InviteRule {
  id: number;
  phase_id: number;
  group_id: number;
  position: number;
}

export interface InviteRuleCondition {
  id: number;
  invite_rule_id: number;
  question_id: number;
  condition_type: 'option_selected' | 'option_not_selected' | 'text_contains' | 'text_not_contains';
  option_id: number | null;
  text_value: string | null;
}

export interface AvailableQuestion {
  id: number;
  phase_id: number;
  phase_name: string;
  text: string;
  question_type: 'text' | 'button';
  options: QuestionOption[];
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
