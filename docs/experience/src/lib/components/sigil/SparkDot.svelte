<script lang="ts">
	import { blueFlame, sparkDotR } from './geometry';

	interface Props {
		opacity: number;
	}

	let { opacity }: Props = $props();
</script>

<circle cx={0} cy={0} r={sparkDotR} fill={blueFlame} {opacity} />
<circle cx={0} cy={0} r={sparkDotR * 0.5} fill="white" opacity={opacity * 0.6} />
