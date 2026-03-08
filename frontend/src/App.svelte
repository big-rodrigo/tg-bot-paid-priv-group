<script lang="ts">
  import { onMount } from 'svelte';
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

  onMount(() => {
    const onHashChange = () => {
      path = window.location.hash.replace(/^#/, '') || '/';
    };
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  });

  let secret = localStorage.getItem('adminSecret') ?? '';
  if (!secret) {
    secret = window.prompt('Enter admin secret:') ?? '';
    localStorage.setItem('adminSecret', secret);
  }
</script>

<nav>
  <strong>TG Bot Admin</strong>
  <a href="/#/">Dashboard</a>
  <a href="/#/phases">Phases</a>
  <a href="/#/groups">Groups</a>
  <a href="/#/users">Users</a>
  <a href="/#/payments">Payments</a>
  <a href="/#/settings">Settings</a>
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
