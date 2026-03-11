#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    PtBr,
}

impl Lang {
    pub fn from_code(s: &str) -> Self {
        match s.to_lowercase().replace('-', "_").as_str() {
            "pt_br" | "pt" => Lang::PtBr,
            _ => Lang::En,
        }
    }

}

// ── Welcome ──────────────────────────────────────────────────────────────────

pub fn already_registered(l: Lang) -> &'static str {
    match l {
        Lang::En => "You are already registered! Check your previous messages for your invite links.\nIf you need help, contact an administrator.",
        Lang::PtBr => "Você já está registrado! Verifique suas mensagens anteriores para seus links de convite.\nSe precisar de ajuda, entre em contato com um administrador.",
    }
}

pub fn resuming_registration(l: Lang) -> &'static str {
    match l {
        Lang::En => "Resuming your registration from where you left off.",
        Lang::PtBr => "Retomando seu cadastro de onde você parou.",
    }
}

pub fn not_configured(l: Lang) -> &'static str {
    match l {
        Lang::En => "Registration is not yet configured. Please check back later.",
        Lang::PtBr => "O cadastro ainda não está configurado. Por favor, tente novamente mais tarde.",
    }
}

pub fn not_started(l: Lang) -> &'static str {
    match l {
        Lang::En => "You haven't started registration yet. Send /start to begin.",
        Lang::PtBr => "Você ainda não iniciou o cadastro. Envie /start para começar.",
    }
}

pub fn status_format(l: Lang, status: &str) -> String {
    match l {
        Lang::En => format!("Registration status: {status}"),
        Lang::PtBr => format!("Status do cadastro: {status}"),
    }
}

pub fn status_not_started(l: Lang) -> &'static str {
    match l {
        Lang::En => "Not started",
        Lang::PtBr => "Não iniciado",
    }
}

pub fn status_registered(l: Lang) -> &'static str {
    match l {
        Lang::En => "Registered ✅",
        Lang::PtBr => "Registrado ✅",
    }
}

pub fn status_in_progress(l: Lang) -> &'static str {
    match l {
        Lang::En => "In progress ⏳",
        Lang::PtBr => "Em andamento ⏳",
    }
}

// ── Registration ─────────────────────────────────────────────────────────────

pub fn send_text(l: Lang) -> &'static str {
    match l {
        Lang::En => "Please send a text message to answer this question.",
        Lang::PtBr => "Por favor, envie uma mensagem de texto para responder esta pergunta.",
    }
}

pub fn send_photo(l: Lang) -> &'static str {
    match l {
        Lang::En => "Please send a photo to answer this question.",
        Lang::PtBr => "Por favor, envie uma foto para responder esta pergunta.",
    }
}

pub fn use_buttons(l: Lang) -> &'static str {
    match l {
        Lang::En => "Please use the buttons below to answer this question.",
        Lang::PtBr => "Por favor, use os botões abaixo para responder esta pergunta.",
    }
}

pub fn photo_prompt(l: Lang) -> &'static str {
    match l {
        Lang::En => "📷 Please send a photo as your answer.",
        Lang::PtBr => "📷 Por favor, envie uma foto como resposta.",
    }
}

pub fn moving_on_to(l: Lang, phase_name: &str) -> String {
    match l {
        Lang::En => format!("Moving on to: <b>{phase_name}</b>"),
        Lang::PtBr => format!("Avançando para: <b>{phase_name}</b>"),
    }
}

// ── Payment ──────────────────────────────────────────────────────────────────

pub fn registration_complete(l: Lang) -> &'static str {
    match l {
        Lang::En => "Registration complete! Please proceed with payment to receive your invite links.",
        Lang::PtBr => "Cadastro completo! Por favor, prossiga com o pagamento para receber seus links de convite.",
    }
}

pub fn pay_livepix(l: Lang) -> &'static str {
    match l {
        Lang::En => "Pay via LivePix",
        Lang::PtBr => "Pagar via LivePix",
    }
}

pub fn gate_rejected(l: Lang) -> &'static str {
    match l {
        Lang::En => "Based on your answers, you are not eligible to proceed. Contact an admin for more information.",
        Lang::PtBr => "Com base nas suas respostas, você não está elegível para prosseguir. Entre em contato com um administrador para mais informações.",
    }
}

pub fn unknown_payment_option(l: Lang) -> &'static str {
    match l {
        Lang::En => "Unknown payment option.",
        Lang::PtBr => "Opção de pagamento desconhecida.",
    }
}

// ── LivePix instructions ─────────────────────────────────────────────────────

pub fn livepix_not_configured(l: Lang) -> &'static str {
    match l {
        Lang::En => "LivePix is not yet configured. Please contact an administrator.",
        Lang::PtBr => "O LivePix ainda não está configurado. Por favor, entre em contato com um administrador.",
    }
}

pub fn livepix_instructions(l: Lang, identifier: &str, currency: &str, price_display: &str, url: &str) -> String {
    match l {
        Lang::En => format!(
            "To complete your payment:\n\
             1. Open the payment page: <a href=\"{url}\">{url}</a>\n\
             2. In the <b>username</b> field, type exactly: <code>{identifier}</code>\n\
             3. Pay at least <b>{currency} {price_display}</b>\n\n\
             Your access links will be sent automatically once the payment is confirmed."
        ),
        Lang::PtBr => format!(
            "Para concluir seu pagamento:\n\
             1. Abra a página de pagamento: <a href=\"{url}\">{url}</a>\n\
             2. No campo <b>nome de usuário</b>, digite exatamente: <code>{identifier}</code>\n\
             3. Pague pelo menos <b>{currency} {price_display}</b>\n\n\
             Seus links de acesso serão enviados automaticamente após a confirmação do pagamento."
        ),
    }
}

// ── Invite delivery ──────────────────────────────────────────────────────────

pub fn payment_confirmed_processing(l: Lang) -> &'static str {
    match l {
        Lang::En => "✅ Payment confirmed! Processing your group invitations...",
        Lang::PtBr => "✅ Pagamento confirmado! Processando seus convites de grupo...",
    }
}

pub fn no_groups_configured(l: Lang) -> &'static str {
    match l {
        Lang::En => "Payment confirmed! No groups are configured yet — an admin will send your links shortly.",
        Lang::PtBr => "Pagamento confirmado! Nenhum grupo configurado ainda — um administrador enviará seus links em breve.",
    }
}

pub fn here_are_links(l: Lang) -> &'static str {
    match l {
        Lang::En => "✅ Payment confirmed! Here are your personal invite links:\n⚠️ Each link can only be used once.",
        Lang::PtBr => "✅ Pagamento confirmado! Aqui estão seus links de convite pessoais:\n⚠️ Cada link só pode ser usado uma vez.",
    }
}

pub fn link_line(title: &str, url: &str) -> String {
    format!("🔗 <b>{title}</b>\n{url}")
}

pub fn link_error(l: Lang, title: &str) -> String {
    match l {
        Lang::En => format!("⚠️ Could not generate link for <b>{title}</b>. An admin will contact you."),
        Lang::PtBr => format!("⚠️ Não foi possível gerar o link para <b>{title}</b>. Um administrador entrará em contato."),
    }
}

pub fn link_error_contact_admin(l: Lang, title: &str) -> String {
    match l {
        Lang::En => format!("⚠️ Could not generate link for <b>{title}</b>. Contact an admin."),
        Lang::PtBr => format!("⚠️ Não foi possível gerar o link para <b>{title}</b>. Entre em contato com um administrador."),
    }
}

pub fn welcome_join(l: Lang) -> &'static str {
    match l {
        Lang::En => "🎉 Welcome! Join the groups using the links above.\nRemember: each link is single-use, so join promptly!",
        Lang::PtBr => "🎉 Bem-vindo! Entre nos grupos usando os links acima.\nLembre-se: cada link é de uso único, então entre logo!",
    }
}

pub fn no_matches(l: Lang) -> &'static str {
    match l {
        Lang::En => "✅ Payment confirmed! Based on your registration, no group invites matched.\nContact an admin if you believe this is an error.",
        Lang::PtBr => "✅ Pagamento confirmado! Com base no seu cadastro, nenhum convite de grupo correspondeu.\nEntre em contato com um administrador se acredita que isso é um erro.",
    }
}

// ── /links & refresh ─────────────────────────────────────────────────────────

pub fn not_registered(l: Lang) -> &'static str {
    match l {
        Lang::En => "You haven't registered yet. Send /start to begin.",
        Lang::PtBr => "Você ainda não se registrou. Envie /start para começar.",
    }
}

pub fn no_payment(l: Lang) -> &'static str {
    match l {
        Lang::En => "You don't have a completed payment yet. Complete registration and payment first.",
        Lang::PtBr => "Você ainda não tem um pagamento confirmado. Complete o cadastro e o pagamento primeiro.",
    }
}

pub fn no_links_available(l: Lang) -> &'static str {
    match l {
        Lang::En => "You have no invite links available. Contact an admin.",
        Lang::PtBr => "Você não tem links de convite disponíveis. Entre em contato com um administrador.",
    }
}

pub fn no_groups_available(l: Lang) -> &'static str {
    match l {
        Lang::En => "No groups are available. Contact an admin.",
        Lang::PtBr => "Nenhum grupo disponível. Entre em contato com um administrador.",
    }
}

pub fn links_header(l: Lang) -> &'static str {
    match l {
        Lang::En => "Here are your invite links:\n⚠️ Each link can only be used once.",
        Lang::PtBr => "Aqui estão seus links de convite:\n⚠️ Cada link só pode ser usado uma vez.",
    }
}

// ── Admin commands ───────────────────────────────────────────────────────────

pub fn admin_no_users(l: Lang) -> &'static str {
    match l {
        Lang::En => "No registered users yet.",
        Lang::PtBr => "Nenhum usuário registrado ainda.",
    }
}

pub fn admin_no_groups(l: Lang) -> &'static str {
    match l {
        Lang::En => "No groups configured. Add them via the web interface.",
        Lang::PtBr => "Nenhum grupo configurado. Adicione-os pela interface web.",
    }
}

pub fn admin_no_phases(l: Lang) -> &'static str {
    match l {
        Lang::En => "No phases configured. Add them via the web interface.",
        Lang::PtBr => "Nenhuma fase configurada. Adicione-as pela interface web.",
    }
}

pub fn admin_usage_sendinvites(l: Lang) -> &'static str {
    match l {
        Lang::En => "Usage: /sendinvites <telegram_id>",
        Lang::PtBr => "Uso: /sendinvites <telegram_id>",
    }
}

pub fn admin_user_not_found(l: Lang) -> &'static str {
    match l {
        Lang::En => "User not found.",
        Lang::PtBr => "Usuário não encontrado.",
    }
}

pub fn admin_invites_sent(l: Lang, telegram_id: i64) -> String {
    match l {
        Lang::En => format!("Invite links sent to user {telegram_id}."),
        Lang::PtBr => format!("Links de convite enviados para o usuário {telegram_id}."),
    }
}

pub fn admin_active(l: Lang) -> &'static str {
    match l {
        Lang::En => "active",
        Lang::PtBr => "ativo",
    }
}

pub fn admin_inactive(l: Lang) -> &'static str {
    match l {
        Lang::En => "inactive",
        Lang::PtBr => "inativo",
    }
}

pub fn admin_no_username(l: Lang) -> &'static str {
    match l {
        Lang::En => "no username",
        Lang::PtBr => "sem username",
    }
}

// ── Backup ───────────────────────────────────────────────────────────────────

pub fn backup_caption(l: Lang, date: &str) -> String {
    match l {
        Lang::En => format!("Database backup — {date}"),
        Lang::PtBr => format!("Backup do banco de dados — {date}"),
    }
}

pub fn backup_part_caption(l: Lang, date: &str, part: usize, total: usize) -> String {
    match l {
        Lang::En => format!("Database backup — {date} (part {part}/{total})"),
        Lang::PtBr => format!("Backup do banco de dados — {date} (parte {part}/{total})"),
    }
}

pub fn backup_failed(l: Lang, err: &str) -> String {
    match l {
        Lang::En => format!("Database backup failed: {err}"),
        Lang::PtBr => format!("Falha no backup do banco de dados: {err}"),
    }
}

// ── Help / command descriptions ──────────────────────────────────────────────

pub fn cmd_start(l: Lang) -> &'static str {
    match l {
        Lang::En => "Start the registration process",
        Lang::PtBr => "Iniciar o processo de cadastro",
    }
}

pub fn cmd_help(l: Lang) -> &'static str {
    match l {
        Lang::En => "Show this help message",
        Lang::PtBr => "Mostrar esta mensagem de ajuda",
    }
}

pub fn cmd_status(l: Lang) -> &'static str {
    match l {
        Lang::En => "Check your registration status",
        Lang::PtBr => "Verificar o status do seu cadastro",
    }
}

pub fn cmd_links(l: Lang) -> &'static str {
    match l {
        Lang::En => "Get your invite links again",
        Lang::PtBr => "Obter seus links de convite novamente",
    }
}

pub fn help_text(l: Lang) -> &'static str {
    match l {
        Lang::En => "\
Available commands:
/start — Start the registration process
/help — Show this help message
/status — Check your registration status
/links — Get your invite links again",
        Lang::PtBr => "\
Comandos disponíveis:
/start — Iniciar o processo de cadastro
/help — Mostrar esta mensagem de ajuda
/status — Verificar o status do seu cadastro
/links — Obter seus links de convite novamente",
    }
}

pub fn admin_help_text(l: Lang) -> &'static str {
    match l {
        Lang::En => "\
Admin commands:
/admin — Show admin help
/users — List all registered users
/groups — List configured groups
/phases — List active phases
/sendinvites <telegram_id> — Send invite links to a user",
        Lang::PtBr => "\
Comandos de administrador:
/admin — Mostrar ajuda do administrador
/users — Listar todos os usuários registrados
/groups — Listar grupos configurados
/phases — Listar fases ativas
/sendinvites <telegram_id> — Enviar links de convite para um usuário",
    }
}
