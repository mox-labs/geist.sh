<script lang="ts">
	import { green, blueFlame, sparkAngles } from './geometry';

	interface Props {
		x: number;
		y: number;
		sz: number;
		op: number;
		rot: number;
		p1: number;
		p2: number;
		p3: number;
		p4: number;
	}

	let { x, y, sz, op, rot, p1, p2, p3, p4 }: Props = $props();

	const RAD = Math.PI / 180;

	let sw = $derived(2.5);
	let cr = $derived(sz * 0.2 * p1);
	let armLen = $derived(sz * 0.5);
	let sparkSz = $derived(sz * 0.12 * p1);
	let sparkR = $derived(sz * 0.07 * p1);
	let sparkW = $derived(sw * 0.5);
</script>

<g transform="translate({x}, {y}) rotate({rot})" opacity={op}>
	{#if p1 > 0}
		<circle cx={0} cy={0} r={cr} fill="none" stroke={green} stroke-width={sw} opacity={p1} />
		{@const a0 = 0}
		<line
			x1={Math.cos(a0) * cr}
			y1={Math.sin(a0) * cr}
			x2={Math.cos(a0) * (cr + armLen * p1)}
			y2={Math.sin(a0) * (cr + armLen * p1)}
			stroke={green}
			stroke-width={sw}
			stroke-linecap="round"
			opacity={p1}
		/>
	{/if}

	{#if p2 > 0}
		{#each [120, 240] as d}
			{@const a = d * RAD}
			<line
				x1={Math.cos(a) * cr}
				y1={Math.sin(a) * cr}
				x2={Math.cos(a) * (cr + armLen * p2)}
				y2={Math.sin(a) * (cr + armLen * p2)}
				stroke={green}
				stroke-width={sw}
				stroke-linecap="round"
				opacity={p2}
			/>
		{/each}
	{/if}

	{#if p3 > 0}
		{#each [60, 180, 300] as d}
			{@const a = d * RAD}
			<line
				x1={Math.cos(a) * cr}
				y1={Math.sin(a) * cr}
				x2={Math.cos(a) * (cr + armLen * p3)}
				y2={Math.sin(a) * (cr + armLen * p3)}
				stroke={green}
				stroke-width={sw}
				stroke-linecap="round"
				opacity={p3}
			/>
		{/each}
	{/if}

	{#if p4 > 0}
		{@const subR = sz * 0.09 * p4}
		{@const subLen = sz * 0.20 * p4}
		{@const subSw = Math.max(0.8, sw * 0.5)}
		{#each [30, 90, 150, 210, 270, 330] as d}
			{@const a = d * RAD}
			<line
				x1={Math.cos(a) * cr}
				y1={Math.sin(a) * cr}
				x2={Math.cos(a) * (cr + armLen * p4)}
				y2={Math.sin(a) * (cr + armLen * p4)}
				stroke={green}
				stroke-width={sw}
				stroke-linecap="round"
				opacity={p4}
			/>
		{/each}
		{#each [0, 30, 60, 90, 120, 150, 180, 210, 240, 270, 300, 330] as d}
			{@const a = d * RAD}
			{@const isNew = d % 60 !== 0}
			{@const tipDist = isNew ? cr + armLen * p4 : cr + armLen}
			{@const tipX = Math.cos(a) * tipDist}
			{@const tipY = Math.sin(a) * tipDist}
			<g transform="translate({tipX},{tipY})" opacity={p4}>
				<circle cx={0} cy={0} r={subR} fill="none" stroke={green} stroke-width={subSw} />
				{#each [0, 120, 240] as sd}
					{@const sa = (sd + d) * RAD}
					<line
						x1={Math.cos(sa) * subR}
						y1={Math.sin(sa) * subR}
						x2={Math.cos(sa) * (subR + subLen)}
						y2={Math.sin(sa) * (subR + subLen)}
						stroke={green}
						stroke-width={subSw}
						stroke-linecap="round"
					/>
				{/each}
			</g>
		{/each}
	{/if}

	<!-- Blue spark — 9 rays matching center design (static) -->
	{#if p1 > 0}
		{#each sparkAngles as a}
			<line
				x1={0}
				y1={0}
				x2={Math.cos(a) * sparkSz}
				y2={Math.sin(a) * sparkSz}
				stroke={blueFlame}
				stroke-width={sparkW}
				stroke-linecap="round"
			/>
		{/each}
		<circle cx={0} cy={0} r={sparkR} fill={blueFlame} />
		<circle cx={0} cy={0} r={sparkR * 0.5} fill="white" opacity={p1 * 0.6} />
	{/if}
</g>
