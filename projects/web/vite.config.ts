import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';

export default defineConfig({
	plugins: [solid()],
	server: {
		watch: {
			usePolling: true
		},
		host: true,
		port: 5173
	},
	build: {
		target: 'esnext'
	},
	optimizeDeps: {
		exclude: ['halvor-wasm']
	}
});

