// Shared geometry for the geist.sh sigil.
// 10% smaller than original (heptR: 55 → 50).

export const V = 400;
export const C = V / 2;

// ── Colors ──
export const green = '#00FF41';
export const red = '#d45555';
export const blueFlame = '#4da6ff';

// ── Heptagon ──
export const heptN = 7;
export const heptR = 50;
export const heptVerts = Array.from({ length: heptN }, (_, i) => {
	const a = (i / heptN) * 2 * Math.PI - Math.PI / 2;
	return { x: Math.cos(a) * heptR, y: Math.sin(a) * heptR };
});

export const heptOutline =
	heptVerts.map((v, i) => `${i === 0 ? 'M' : 'L'} ${v.x} ${v.y}`).join(' ') + ' Z';

const starVerts = Array.from({ length: heptN }, (_, i) => heptVerts[(i * 2) % 7]);
export const heptStar =
	starVerts.map((v, i) => `${i === 0 ? 'M' : 'L'} ${v.x} ${v.y}`).join(' ') + ' Z';

// Perimeters (computed, rounded up for dasharray safety)
export const heptPerimeter = Math.ceil(heptN * 2 * heptR * Math.sin(Math.PI / heptN)) + 5;
export const starPerimeter = Math.ceil(heptN * 2 * heptR * Math.sin((2 * Math.PI) / heptN)) + 5;

// ── Treya (triskelion) ──
export const starApothem = heptR * Math.cos((2 * Math.PI) / heptN);
export const treyaR = starApothem - 3; // inset for visual clearance
export const treyaScale = treyaR / 19;
export const treyaArmEnd = 50 * treyaScale;
export const treyaAngles = [0, 120, 240].map((d) => (d - 90) * (Math.PI / 180));

// d-shape: 120° arc segment in local space (arm pointing along +x)
export const dArcPath = (() => {
	const a0 = -Math.PI / 3;
	const a1 = Math.PI / 3;
	return `M ${Math.cos(a0) * treyaR} ${Math.sin(a0) * treyaR} A ${treyaR} ${treyaR} 0 0 1 ${Math.cos(a1) * treyaR} ${Math.sin(a1) * treyaR}`;
})();

// ── Spark ──
export const sparkRayCount = 9;
export const sparkAngles = Array.from(
	{ length: sparkRayCount },
	(_, i) => (i * 40 - 90) * (Math.PI / 180)
);
export const sparkDotR = 5;
export const sparkSw = 1.4;

// Per-ray flicker seeds — slow, organic, independent per ray.
// Max length = treyaR (touches green circle). Min varies per ray.
export const sparkRaySeeds = Array.from({ length: sparkRayCount }, (_, i) => ({
	freq1: 0.001 + i * 0.00025,
	freq2: 0.0018 + i * 0.00042,
	phase: i * 2.3 + 0.7,
	minLen: 3 + ((i * 7) % 11),
	maxLen: treyaR
}));

// ── Arm assembly definitions ──
export const armDefs = [
	{ finalDeg: -90, startDeg: 0, startX: 130, startY: -100 },
	{ finalDeg: 30, startDeg: 180, startX: -135, startY: 25 },
	{ finalDeg: 150, startDeg: 270, startX: -10, startY: 150 }
];

// ── Orbit (running state) ──
export const spiralStart = 95;
export const spiralEnd = 160;
export const spiralTurns = 1.0;
