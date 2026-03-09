<script lang="ts">
  import { onMount } from 'svelte';
  import { t, setLang } from './lib/i18n.svelte';
  import { settings } from './lib/api';
  import Dashboard from './pages/Dashboard.svelte';
  import Phases from './pages/Phases.svelte';
  import Groups from './pages/Groups.svelte';
  import Users from './pages/Users.svelte';
  import Payments from './pages/Payments.svelte';
  import Settings from './pages/Settings.svelte';

  const routes: Record<string, any> = {
    '/': Dashboard,
    '/phases': Phases,
    '/groups': Groups,
    '/users': Users,
    '/payments': Payments,
    '/settings': Settings,
  };

  let path = $state(window.location.hash.replace(/^#/, '') || '/');
  let CurrentPage = $derived(routes[path] ?? Dashboard);

  onMount(async () => {
    const onHashChange = () => {
      path = window.location.hash.replace(/^#/, '') || '/';
    };
    window.addEventListener('hashchange', onHashChange);

    // Load language setting
    try {
      const langSetting = await settings.get('language');
      setLang(langSetting.value as 'en' | 'pt-BR');
    } catch {
      // Default to English
    }

    return () => window.removeEventListener('hashchange', onHashChange);
  });

  let secret = localStorage.getItem('adminSecret') ?? '';
  if (!secret) {
    secret = window.prompt(t('auth.prompt')) ?? '';
    localStorage.setItem('adminSecret', secret);
  }
</script>

<nav>
  <strong>{t('nav.title')}</strong>
  <a href="/#/">{t('nav.dashboard')}</a>
  <a href="/#/phases">{t('nav.phases')}</a>
  <a href="/#/groups">{t('nav.groups')}</a>
  <a href="/#/users">{t('nav.users')}</a>
  <a href="/#/payments">{t('nav.payments')}</a>
  <a href="/#/settings">{t('nav.settings')}</a>
</nav>

<main>
  <CurrentPage />
</main>

<style>
  nav {
    display: flex;
    gap: 1rem;
    align-items: center;
    padding: 0.75rem 1.5rem;
    background: #1a1a2e;
    color: white;
  }
  nav strong {
    margin-right: auto;
  }
  nav a {
    color: #aad4f5;
    text-decoration: none;
    font-size: 0.95rem;
  }
  nav a:hover {
    color: white;
  }
  main {
    padding: 1.5rem;
    max-width: 1100px;
    margin: 0 auto;
  }
</style>
