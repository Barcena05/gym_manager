<script lang="ts">
	import { onMount } from 'svelte';
	import jsBarcode from 'jsbarcode';

	interface Props {
		value: string;
		name?: string;
		height?: number;
	}

	let { value, name = '', height = 60 }: Props = $props();
	let svgRef: SVGSVGElement | null = null;

	function render() {
		if (!svgRef || !value) return;
		try {
			jsBarcode(svgRef, value, {
				format: 'CODE128',
				width: 1.6,
				height: height / 40,
				displayValue: true,
				margin: 8,
				background: '#ffffff',
				lineColor: '#000000',
				font: 'Arial',
				fontSize: 14,
				textMargin: 1,
				valid: () => true
			});
		} catch (e) {
			console.error('Error generando código de barras:', e);
		}
	}

	onMount(render);
	$effect(() => {
		if (svgRef && value) render();
	});
</script>

<div class="barcode-label flex flex-col items-center border rounded p-2 bg-white text-black break-inside-avoid">
	{#if name}
		<span class="text-xs font-semibold text-center mb-1 leading-tight">{name}</span>
	{/if}
	<svg bind:this={svgRef} class="max-w-full h-auto"></svg>
</div>
