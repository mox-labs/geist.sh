import { defineCollection, defineConfig } from '@content-collections/core';
import { z } from 'zod';

const docs = defineCollection({
	name: 'docs',
	directory: '../content',
	include: '*.md',
	schema: z.object({
		title: z.string(),
		description: z.string().optional(),
		order: z.number().optional(),
		section: z.enum(['overview', 'architecture', 'reference']).optional(),
		content: z.string()
	}),
	transform: (doc) => ({
		...doc,
		slug: doc._meta.path.replace(/\.md$/, '')
	})
});

const decisions = defineCollection({
	name: 'decisions',
	directory: '../content/decisions',
	include: '*.md',
	schema: z.object({
		title: z.string(),
		date: z.string(),
		status: z.enum(['proposed', 'accepted', 'superseded', 'deprecated']).optional(),
		content: z.string()
	}),
	transform: (doc) => ({
		...doc,
		slug: doc._meta.path.replace(/\.md$/, '')
	})
});

export default defineConfig({
	collections: [docs, decisions]
});
