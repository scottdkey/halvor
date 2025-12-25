import { Component, createSignal, onMount } from 'solid-js';
import { api, type DiscoveredHost } from './lib/api';
import './App.css';

const App: Component = () => {
  const [hosts, setHosts] = createSignal<DiscoveredHost[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const discoverAgents = async () => {
    setLoading(true);
    setError(null);

    try {
      // Call Rust API server
      const discovered = await api.discoverAgents();
      setHosts(discovered);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    // Optionally auto-discover on mount
  });

  return (
    <>
      <head>
        <title>Halvor - Agent Discovery</title>
      </head>
      <main>
        <h1>Halvor Agent Discovery</h1>

        <button onClick={discoverAgents} disabled={loading()}>
          {loading() ? 'Discovering...' : 'Discover Agents'}
        </button>

        {error() && <div class="error">{error()}</div>}

        {hosts().length > 0 && (
          <ul>
            {hosts().map((host) => (
              <li>
                <strong>{host.hostname}</strong>
                {host.localIp && ` - ${host.localIp}`}
                {host.reachable ? ' ✓' : ' ✗'}
              </li>
            ))}
          </ul>
        )}
      </main>
    </>
  );
};

export default App;

