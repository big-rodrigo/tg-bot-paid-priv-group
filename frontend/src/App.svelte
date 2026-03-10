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
  let navOpen = $state(false);

  onMount(() => {
    const onHashChange = () => {
      path = window.location.hash.replace(/^#/, '') || '/';
    };
    window.addEventListener('hashchange', onHashChange);

    // Load language setting (fire-and-forget)
    settings.get('admin_language').then((langSetting) => {
      setLang(langSetting.value as 'en' | 'pt-BR');
    }).catch(() => {
      // Default to English
    });

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
  <button class="hamburger" aria-label="Toggle menu" onclick={() => navOpen = !navOpen}>
    {navOpen ? '✕' : '☰'}
  </button>
  <div class="nav-links" class:open={navOpen}>
    <a href="/#/" onclick={() => navOpen = false}>{t('nav.dashboard')}</a>
    <a href="/#/phases" onclick={() => navOpen = false}>{t('nav.phases')}</a>
    <a href="/#/groups" onclick={() => navOpen = false}>{t('nav.groups')}</a>
    <a href="/#/users" onclick={() => navOpen = false}>{t('nav.users')}</a>
    <a href="/#/payments" onclick={() => navOpen = false}>{t('nav.payments')}</a>
    <a href="/#/settings" onclick={() => navOpen = false}>{t('nav.settings')}</a>
  </div>
</nav>

<main>
  <CurrentPage />
</main>

<style>
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    align-items: center;
    padding: 0.75rem 1.5rem;
    background: #1a1a2e;
    color: white;
    position: relative;
  }
  nav strong {
    flex: 1;
  }
  .hamburger {
    display: none;
    background: transparent;
    border: none;
    color: white;
    font-size: 1.3rem;
    cursor: pointer;
    padding: 0.2rem 0.4rem;
    line-height: 1;
  }
  .nav-links {
    display: flex;
    gap: 1rem;
    align-items: center;
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

  @media (max-width: 640px) {
    nav {
      padding: 0.6rem 1rem;
    }
    .hamburger {
      display: block;
    }
    .nav-links {
      display: none;
      width: 100%;
      flex-direction: column;
      align-items: flex-start;
      gap: 0;
      padding-bottom: 0.5rem;
    }
    .nav-links.open {
      display: flex;
    }
    .nav-links a {
      padding: 0.5rem 0;
      width: 100%;
      border-top: 1px solid rgba(255,255,255,0.08);
      font-size: 1rem;
    }
    main {
      padding: 1rem;
    }
  }
</style>
