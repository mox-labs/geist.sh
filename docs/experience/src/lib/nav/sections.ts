import type { NavSection } from './types';

export const sections: NavSection[] = [
	{
		label: 'Overview',
		items: [
			{ label: 'What is geist.sh', href: '/docs/overview' },
			{ label: 'Architecture', href: '/docs/architecture' },
		]
	},
	{
		label: 'Concepts',
		items: [
			{ label: 'Capability-Led Connectivity', href: '/docs/capability-led-connectivity' },
		]
	},
	{
		label: 'Reference',
		items: [
			{ label: 'Status', href: '/docs/status' },
			{ label: 'Decisions', href: '/docs/decisions' },
		]
	}
];
