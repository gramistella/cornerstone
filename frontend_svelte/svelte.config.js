import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({
			// The output directory for the built files
			pages: 'build',
			assets: 'build',
			fallback: 'index.html', // Essential for SPA routing
			precompress: false,
			strict: true
		}),
		alias: {
			'$lib': 'src/lib',
		}
	}
};

export default config;