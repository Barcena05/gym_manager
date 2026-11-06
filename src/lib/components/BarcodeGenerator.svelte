<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import jsBarcode from 'jsbarcode';
	import Button from '$lib/components/ui/button/button.svelte';
	import * as Card from '$lib/components/ui/card';
	import Label from '$lib/components/ui/label/label.svelte';
	import Input from '$lib/components/ui/input/input.svelte';
	import { printElementById } from '$lib/print';

	interface Props {
		/** Valor a codificar (ej. el card_id del miembro) */
		value?: string;
		/** Alto de las barras en px */
		height?: number;
		/** Mostrar el valor debajo del código */
		displayValue?: boolean;
		/** Modo etiqueta: sin cabecera, sin input y sin botones de edición */
		readonly?: boolean;
		/** Mostrar el botón de imprimir dentro del componente */
		showPrintButton?: boolean;
		/** Nombre que se muestra encima del código en modo etiqueta */
		name?: string;
	}

	let {
		value = '',
		height = 70,
		displayValue = true,
		readonly = false,
		showPrintButton = true,
		name = ''
	}: Props = $props();

	let svgRef: SVGSVGElement | null = $state(null);
	let inputValue = $state(value);
	let isGenerating = $state(false);
	let generateError = $state<string | null>(null);

	function generateBarcode(val: string) {
		if (!svgRef || !val) return;
		isGenerating = true;
		generateError = null;
		try {
			jsBarcode(svgRef, val, {
				format: 'CODE128',
				width: 2,
				height,
				displayValue,
				margin: 12,
				background: '#ffffff',
				lineColor: '#000000',
				font: 'Arial',
				textAlign: 'center',
				textPosition: displayValue ? 'bottom' : 'none',
				textMargin: 2,
				fontSize: 16,
				valid: () => true
			});

			// jsBarcode already writes the correct width, height and viewBox.
			// Do not overwrite the viewBox here: doing so shifts the barcode
			// to the left inside the SVG viewport.
		} catch (e) {
			console.error('Error generando código de barras:', e);
			generateError = m['barcode.generation_failed']() || 'No se pudo generar el código de barras';
		} finally {
			isGenerating = false;
		}
	}

	function handleInputChange(e: Event) {
		const target = e.target as HTMLInputElement;
		inputValue = target.value;
	}

	// Mantener sincronizado el valor cuando el miembro cambia.
	$effect(() => {
		inputValue = value;
	});

	// Regenerar cuando cambie el valor o cuando el SVG ya esté montado.
	$effect(() => {
		if (inputValue && svgRef) {
			generateBarcode(inputValue);
		}
	});
</script>

<Card.Root class="w-full max-w-md mx-auto">
	{#if !readonly}
		<Card.Header class="flex flex-col items-center gap-2">
			<Card.Title class="text-lg">{m['barcode.title']()}</Card.Title>
			<Card.Description>{m['barcode.description']()}</Card.Description>
		</Card.Header>
	{/if}

	<Card.Content class="flex flex-col items-center justify-center p-3 sm:p-6">
		{#if isGenerating}
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
		{:else if generateError}
			<div class="text-sm text-destructive text-center p-4 bg-destructive/10 rounded">
				{generateError}
			</div>
		{:else}
			<div
				id="barcode-print-area"
				class="w-full flex flex-col items-center justify-center overflow-x-auto"
			>
				{#if name}
					<p class="text-center text-sm font-semibold mb-2">{name}</p>
				{/if}

				<div class="w-full max-w-[360px] mx-auto">
					<svg
						bind:this={svgRef}
						class="w-full h-auto max-h-[160px]"
						preserveAspectRatio="xMidYMid meet"
					/>
				</div>

				{#if readonly && value}
					<p class="mt-2 text-center font-mono text-sm">{value}</p>
				{/if}
			</div>
		{/if}
	</Card.Content>

	{#if !readonly}
		<Card.Footer class="flex flex-col gap-2 p-4">
			<Label for="barcode-value-input" class="font-semibold text-sm">
				{m['barcode.value_label']()}
			</Label>
			<Input
				id="barcode-value-input"
				type="text"
				class="w-full text-center text-base sm:text-lg font-mono bg-muted border border-input rounded px-2 py-2"
				bind:value={inputValue}
				oninput={handleInputChange}
			/>
			<div class="flex flex-wrap gap-2 justify-center">
				<Button
					variant="outline"
					size="sm"
					onclick={() => generateBarcode(inputValue)}
					disabled={isGenerating || !inputValue}
				>
					{m['barcode.regenerate']()}
				</Button>
				{#if showPrintButton}
					<Button
						variant="secondary"
						size="sm"
						onclick={() => printElementById('barcode-print-area')}
					>
						{m['barcode.print']()}
					</Button>
				{/if}
			</div>
			<p class="text-xs text-muted-foreground text-center">
				{m['barcode.scan_instruction']()}
			</p>
		</Card.Footer>
	{/if}
</Card.Root>

