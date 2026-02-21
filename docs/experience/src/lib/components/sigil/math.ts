/** Clamp t to [0,1] over a time range. */
export function t01(now: number, start: number, end: number): number {
	return Math.max(0, Math.min(1, (now - start) / (end - start)));
}

export function easeOut(t: number): number {
	return 1 - (1 - t) * (1 - t);
}

export function easeOutBack(t: number): number {
	const c = 1.70158;
	return 1 + (c + 1) * Math.pow(t - 1, 3) + c * Math.pow(t - 1, 2);
}

export function lerp(a: number, b: number, t: number): number {
	return a + (b - a) * t;
}

export function phase(p: number, start: number, end: number): number {
	return Math.max(0, Math.min(1, (p - start) / (end - start)));
}
