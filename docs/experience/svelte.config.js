import adapter from '@sveltejs/adapter-cloudflare';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter(),
		alias: {
			'content-collections': '.content-collections/generated'
		},
		prerender: {
			handleUnseenRoutes: 'ignore'
		}
	}
};

export default config;
