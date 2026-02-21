import { allDocs } from 'content-collections';
import { error } from '@sveltejs/kit';

export function load({ params }: { params: { slug: string } }) {
	const doc = allDocs.find((d) => d.slug === params.slug);
	if (!doc) throw error(404, 'Not found');
	return { doc };
}
