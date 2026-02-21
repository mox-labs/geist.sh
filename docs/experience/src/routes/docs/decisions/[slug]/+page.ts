import { allDecisions } from 'content-collections';
import { error } from '@sveltejs/kit';

export function load({ params }: { params: { slug: string } }) {
	const decision = allDecisions.find((d) => d.slug === params.slug);
	if (!decision) throw error(404, 'Not found');
	return { decision };
}
