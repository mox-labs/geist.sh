export interface NavItem {
	label: string;
	href: string;
}

export interface NavSection {
	label: string;
	items: NavItem[];
}
