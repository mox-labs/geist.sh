import { browser } from '$app/environment';

const STORAGE_KEY = 'geist-visited';

let _visited = $state(false);

if (browser) {
	_visited = localStorage.getItem(STORAGE_KEY) === '1';
}

export function hasVisited(): boolean {
	return _visited;
}

export function markVisited(): void {
	if (browser) {
		_visited = true;
		localStorage.setItem(STORAGE_KEY, '1');
	}
}
