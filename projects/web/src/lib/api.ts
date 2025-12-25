// API client for Rust web server
const API_BASE = import.meta.env.VITE_API_URL || '/api';

export interface DiscoveredHost {
  hostname?: string;
  localIp?: string;
  tailscaleIp?: string;
  tailscaleHostname?: string;
  agentPort?: number;
  reachable?: boolean;
}

export interface HostInfo {
  dockerVersion?: string;
  tailscaleInstalled?: boolean;
  portainerInstalled?: boolean;
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

async function apiCall<T>(endpoint: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${endpoint}`, {
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    ...options,
  });

  if (!response.ok) {
    throw new Error(`API error: ${response.statusText}`);
  }

  const result: ApiResponse<T> = await response.json();

  if (!result.success) {
    throw new Error(result.error || 'Unknown API error');
  }

  return result.data!;
}

export const api = {
  async discoverAgents(): Promise<DiscoveredHost[]> {
    return apiCall<DiscoveredHost[]>('/discover-agents');
  },

  async discoverViaTailscale(): Promise<DiscoveredHost[]> {
    return apiCall<DiscoveredHost[]>('/discover-tailscale');
  },

  async discoverViaLocalNetwork(): Promise<DiscoveredHost[]> {
    return apiCall<DiscoveredHost[]>('/discover-local');
  },

  async pingAgent(host: string, port: number): Promise<boolean> {
    return apiCall<boolean>('/ping-agent', {
      method: 'POST',
      body: JSON.stringify({ host, port }),
    });
  },

  async getHostInfo(host: string, port: number): Promise<HostInfo> {
    return apiCall<HostInfo>('/host-info', {
      method: 'POST',
      body: JSON.stringify({ host, port }),
    });
  },

  async executeCommand(
    host: string,
    port: number,
    command: string,
    args: string[]
  ): Promise<string> {
    return apiCall<string>('/execute-command', {
      method: 'POST',
      body: JSON.stringify({ host, port, command, args }),
    });
  },

  async getVersion(): Promise<string> {
    return apiCall<string>('/version');
  },
};
