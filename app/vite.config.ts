import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { config as dotenvConfig } from 'dotenv';
import { defineConfig } from 'vite';

export default defineConfig(({ mode }) => {
	const dev = mode === "development";
	dotenvConfig({ path: dev ? ".env.development" : ".env.production" });
	const port = process.env.PORT ? Number(process.env.PORT) : 3000;
	return {
		plugins: [
			tailwindcss(),
			sveltekit()
		],
		clearScreen: !dev,
		server: {
			port,
			strictPort: true,
			host: dev ? "0.0.0.0" : "localhost",
		},
	};
});

