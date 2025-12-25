
// this file is generated — do not edit it


/// <reference types="@sveltejs/kit" />

/**
 * Environment variables [loaded by Vite](https://vitejs.dev/guide/env-and-mode.html#env-files) from `.env` files and `process.env`. Like [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private), this module cannot be imported into client-side code. This module only includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://svelte.dev/docs/kit/configuration#env) (if configured).
 * 
 * _Unlike_ [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private), the values exported from this module are statically injected into your bundle at build time, enabling optimisations like dead code elimination.
 * 
 * ```ts
 * import { API_KEY } from '$env/static/private';
 * ```
 * 
 * Note that all environment variables referenced in your code should be declared (for example in an `.env` file), even if they don't have a value until the app is deployed:
 * 
 * ```
 * MY_FEATURE_FLAG=""
 * ```
 * 
 * You can override `.env` values from the command line like so:
 * 
 * ```sh
 * MY_FEATURE_FLAG="enabled" npm run dev
 * ```
 */
declare module '$env/static/private' {
	export const GITHUB_TOKEN: string;
	export const SSH_CLIENT: string;
	export const NGINX_PROXY_MANAGER_PASSWORD: string;
	export const USER: string;
	export const SMB_WILLOW_PASSWORD: string;
	export const npm_config_user_agent: string;
	export const HOST_OAK_IP: string;
	export const GIT_ASKPASS: string;
	export const XDG_SESSION_TYPE: string;
	export const npm_node_execpath: string;
	export const HOST_MINT_HOSTNAME: string;
	export const SMB_WILLOW_HOST: string;
	export const SHLVL: string;
	export const BROWSER: string;
	export const npm_config_noproxy: string;
	export const MOTD_SHOWN: string;
	export const HOST_BAULDER_SUDO_PASS: string;
	export const HOME: string;
	export const OLDPWD: string;
	export const NVM_BIN: string;
	export const VSCODE_IPC_HOOK_CLI: string;
	export const TERM_PROGRAM_VERSION: string;
	export const npm_package_json: string;
	export const NVM_INC: string;
	export const NGINX_PROXY_MANAGER_URL: string;
	export const SMB_MAPLE_PASSWORD: string;
	export const npm_package_engines_node: string;
	export const HOST_BAULDER_HOSTNAME: string;
	export const VSCODE_GIT_ASKPASS_MAIN: string;
	export const DIRENV_DIFF: string;
	export const SSL_CERT_FILE: string;
	export const VSCODE_GIT_ASKPASS_NODE: string;
	export const npm_config_userconfig: string;
	export const npm_config_local_prefix: string;
	export const SMB_MAPLE_HOST: string;
	export const MAKEFLAGS: string;
	export const DBUS_SESSION_BUS_ADDRESS: string;
	export const GITHUB_USER: string;
	export const COLORTERM: string;
	export const COLOR: string;
	export const NVM_DIR: string;
	export const MAKE_TERMERR: string;
	export const HOST_BAULDER_SUDO_USER: string;
	export const APP_STORE_CONNECT_API_KEY_ID: string;
	export const LOGNAME: string;
	export const DIRENV_FILE: string;
	export const _: string;
	export const npm_config_prefix: string;
	export const npm_config_npm_version: string;
	export const K3S_TOKEN: string;
	export const PIA_USERNAME: string;
	export const XDG_SESSION_CLASS: string;
	export const APP_STORE_CONNECT_API_KEY_PATH: string;
	export const ACME_EMAIL: string;
	export const HOST_FRIGG_SUDO_PASS: string;
	export const TERM: string;
	export const XDG_SESSION_ID: string;
	export const npm_config_cache: string;
	export const HOST_MINT_IP: string;
	export const APP_STORE_CONNECT_API_ISSUER: string;
	export const npm_config_node_gyp: string;
	export const PATH: string;
	export const DIRENV_WATCHES: string;
	export const HOST_FRIGG_HOSTNAME: string;
	export const NODE: string;
	export const npm_package_name: string;
	export const SMB_WILLOW_USERNAME: string;
	export const NGINX_PROXY_MANAGER_USERNAME: string;
	export const MAKELEVEL: string;
	export const XDG_RUNTIME_DIR: string;
	export const SSL_CERT_DIR: string;
	export const HOST_BAULDER_IP: string;
	export const LANG: string;
	export const SMB_WILLOW_SHARES: string;
	export const VSCODE_GIT_IPC_HANDLE: string;
	export const TERM_PROGRAM: string;
	export const LS_COLORS: string;
	export const npm_lifecycle_script: string;
	export const HOST_FRIGG_SUDO_USER: string;
	export const HOST_OAK_SUDO_PASS: string;
	export const SHELL: string;
	export const npm_package_version: string;
	export const npm_lifecycle_event: string;
	export const TAILNET_BASE: string;
	export const SMB_MAPLE_USERNAME: string;
	export const MAKE_TERMOUT: string;
	export const HOST_OAK_HOSTNAME: string;
	export const HALVOR_ENV: string;
	export const DIRENV_DIR: string;
	export const SMB_MAPLE_SHARES: string;
	export const HOST_MINT_TAILSCALE_IP: string;
	export const VSCODE_GIT_ASKPASS_EXTRA_ARGS: string;
	export const VSCODE_GIT_IPC_AUTH_TOKEN: string;
	export const npm_config_globalconfig: string;
	export const npm_config_init_module: string;
	export const LC_ALL: string;
	export const PWD: string;
	export const npm_execpath: string;
	export const NVM_CD_FLAGS: string;
	export const HOST_OAK_SUDO_USER: string;
	export const SSH_CONNECTION: string;
	export const npm_config_global_prefix: string;
	export const OP_SESSION_KEQJCIJ2BJAVJGWNR26FEJBYAE: string;
	export const HOST_FRIGG_IP: string;
	export const npm_command: string;
	export const MFLAGS: string;
	export const PIA_PASSWORD: string;
	export const PRIVATE_TLD: string;
	export const APP_STORE_CONNECT_TEAM_ID: string;
	export const INIT_CWD: string;
	export const EDITOR: string;
}

/**
 * Similar to [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private), except that it only includes environment variables that begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) (which defaults to `PUBLIC_`), and can therefore safely be exposed to client-side code.
 * 
 * Values are replaced statically at build time.
 * 
 * ```ts
 * import { PUBLIC_BASE_URL } from '$env/static/public';
 * ```
 */
declare module '$env/static/public' {
	export const PUBLIC_TLD: string;
}

/**
 * This module provides access to runtime environment variables, as defined by the platform you're running on. For example if you're using [`adapter-node`](https://github.com/sveltejs/kit/tree/main/packages/adapter-node) (or running [`vite preview`](https://svelte.dev/docs/kit/cli)), this is equivalent to `process.env`. This module only includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://svelte.dev/docs/kit/configuration#env) (if configured).
 * 
 * This module cannot be imported into client-side code.
 * 
 * ```ts
 * import { env } from '$env/dynamic/private';
 * console.log(env.DEPLOYMENT_SPECIFIC_VARIABLE);
 * ```
 * 
 * > [!NOTE] In `dev`, `$env/dynamic` always includes environment variables from `.env`. In `prod`, this behavior will depend on your adapter.
 */
declare module '$env/dynamic/private' {
	export const env: {
		GITHUB_TOKEN: string;
		SSH_CLIENT: string;
		NGINX_PROXY_MANAGER_PASSWORD: string;
		USER: string;
		SMB_WILLOW_PASSWORD: string;
		npm_config_user_agent: string;
		HOST_OAK_IP: string;
		GIT_ASKPASS: string;
		XDG_SESSION_TYPE: string;
		npm_node_execpath: string;
		HOST_MINT_HOSTNAME: string;
		SMB_WILLOW_HOST: string;
		SHLVL: string;
		BROWSER: string;
		npm_config_noproxy: string;
		MOTD_SHOWN: string;
		HOST_BAULDER_SUDO_PASS: string;
		HOME: string;
		OLDPWD: string;
		NVM_BIN: string;
		VSCODE_IPC_HOOK_CLI: string;
		TERM_PROGRAM_VERSION: string;
		npm_package_json: string;
		NVM_INC: string;
		NGINX_PROXY_MANAGER_URL: string;
		SMB_MAPLE_PASSWORD: string;
		npm_package_engines_node: string;
		HOST_BAULDER_HOSTNAME: string;
		VSCODE_GIT_ASKPASS_MAIN: string;
		DIRENV_DIFF: string;
		SSL_CERT_FILE: string;
		VSCODE_GIT_ASKPASS_NODE: string;
		npm_config_userconfig: string;
		npm_config_local_prefix: string;
		SMB_MAPLE_HOST: string;
		MAKEFLAGS: string;
		DBUS_SESSION_BUS_ADDRESS: string;
		GITHUB_USER: string;
		COLORTERM: string;
		COLOR: string;
		NVM_DIR: string;
		MAKE_TERMERR: string;
		HOST_BAULDER_SUDO_USER: string;
		APP_STORE_CONNECT_API_KEY_ID: string;
		LOGNAME: string;
		DIRENV_FILE: string;
		_: string;
		npm_config_prefix: string;
		npm_config_npm_version: string;
		K3S_TOKEN: string;
		PIA_USERNAME: string;
		XDG_SESSION_CLASS: string;
		APP_STORE_CONNECT_API_KEY_PATH: string;
		ACME_EMAIL: string;
		HOST_FRIGG_SUDO_PASS: string;
		TERM: string;
		XDG_SESSION_ID: string;
		npm_config_cache: string;
		HOST_MINT_IP: string;
		APP_STORE_CONNECT_API_ISSUER: string;
		npm_config_node_gyp: string;
		PATH: string;
		DIRENV_WATCHES: string;
		HOST_FRIGG_HOSTNAME: string;
		NODE: string;
		npm_package_name: string;
		SMB_WILLOW_USERNAME: string;
		NGINX_PROXY_MANAGER_USERNAME: string;
		MAKELEVEL: string;
		XDG_RUNTIME_DIR: string;
		SSL_CERT_DIR: string;
		HOST_BAULDER_IP: string;
		LANG: string;
		SMB_WILLOW_SHARES: string;
		VSCODE_GIT_IPC_HANDLE: string;
		TERM_PROGRAM: string;
		LS_COLORS: string;
		npm_lifecycle_script: string;
		HOST_FRIGG_SUDO_USER: string;
		HOST_OAK_SUDO_PASS: string;
		SHELL: string;
		npm_package_version: string;
		npm_lifecycle_event: string;
		TAILNET_BASE: string;
		SMB_MAPLE_USERNAME: string;
		MAKE_TERMOUT: string;
		HOST_OAK_HOSTNAME: string;
		HALVOR_ENV: string;
		DIRENV_DIR: string;
		SMB_MAPLE_SHARES: string;
		HOST_MINT_TAILSCALE_IP: string;
		VSCODE_GIT_ASKPASS_EXTRA_ARGS: string;
		VSCODE_GIT_IPC_AUTH_TOKEN: string;
		npm_config_globalconfig: string;
		npm_config_init_module: string;
		LC_ALL: string;
		PWD: string;
		npm_execpath: string;
		NVM_CD_FLAGS: string;
		HOST_OAK_SUDO_USER: string;
		SSH_CONNECTION: string;
		npm_config_global_prefix: string;
		OP_SESSION_KEQJCIJ2BJAVJGWNR26FEJBYAE: string;
		HOST_FRIGG_IP: string;
		npm_command: string;
		MFLAGS: string;
		PIA_PASSWORD: string;
		PRIVATE_TLD: string;
		APP_STORE_CONNECT_TEAM_ID: string;
		INIT_CWD: string;
		EDITOR: string;
		[key: `PUBLIC_${string}`]: undefined;
		[key: `${string}`]: string | undefined;
	}
}

/**
 * Similar to [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private), but only includes variables that begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) (which defaults to `PUBLIC_`), and can therefore safely be exposed to client-side code.
 * 
 * Note that public dynamic environment variables must all be sent from the server to the client, causing larger network requests — when possible, use `$env/static/public` instead.
 * 
 * ```ts
 * import { env } from '$env/dynamic/public';
 * console.log(env.PUBLIC_DEPLOYMENT_SPECIFIC_VARIABLE);
 * ```
 */
declare module '$env/dynamic/public' {
	export const env: {
		PUBLIC_TLD: string;
		[key: `PUBLIC_${string}`]: string | undefined;
	}
}
