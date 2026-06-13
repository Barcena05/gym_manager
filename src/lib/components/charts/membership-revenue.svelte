<script lang="ts">
	import * as Chart from '$lib/components/ui/chart/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { scaleBand } from 'd3-scale';
	import { BarChart } from 'layerchart';
	import { cubicInOut } from 'svelte/easing';
	import { m } from '$lib/paraglide/messages';
	
	// 👇 Importar el store del tipo de cambio
	import { exchangeRate } from '$lib/stores/settings';

	let {
		chartData = [],
		chartConfig = {}
	}: {
		chartData: { membership_type_name: string; total_revenue: number; count: number, color: string }[];
		chartConfig: Chart.ChartConfig;
	} = $props();

	// 👇 Tipo de cambio actual
	let currentRate = $state(660);
	exchangeRate.subscribe(rate => {
		currentRate = rate;
	});

	const totalUSD = $derived.by(() => {
		return chartData.reduce((acc, curr) => acc + curr.total_revenue, 0);
	});

	// 👇 Total en CUP calculado a partir del total en USD
	const totalCUP = $derived(totalUSD * currentRate);
</script>

<Card.Root class="flex flex-col 2xl:w-1/3 w-full h-[500px] shrink-0">
	<Card.Header class="items-center">
		<Card.Title>{m.membership_revenue()}</Card.Title>
		<Card.Description>
			{m.membership_revenue_desc()}
			<!-- 👇 Aclaración opcional para que el usuario sepa que el gráfico está en USD -->
			<span class="block text-xs text-muted-foreground mt-1">(Montos en USD)</span>
		</Card.Description>
	</Card.Header>
	<Card.Content class="flex items-center justify-center">
		<Chart.Container config={chartConfig} class="mx-auto aspect-square h-[300px] ">
			<BarChart
				data={chartData}
				orientation="horizontal"
				yScale={scaleBand().padding(0.25)}
				y="membership_type_name"
				x="total_revenue"
				cRange={chartData.map((c) => c.color)}
				c="color"
				padding={{ left: 48 }}
				grid={false}
				rule={false}
				axis="y"
				props={{
					bars: {
						stroke: 'none',
						radius: 5,
						rounded: 'all',
						initialWidth: 0,
						initialX: 0,
						motion: {
							x: { type: 'tween', duration: 500, easing: cubicInOut },
							width: { type: 'tween', duration: 500, easing: cubicInOut }
						}
					},
					highlight: { area: { fill: 'none' } },
					yAxis: {
						format: (d) => chartConfig[d as keyof typeof chartConfig].label,
						tickLabelProps: {
							svgProps: {
								x: -16
							}
						}
					}
				}}
			>
				{#snippet tooltip()}
					<!-- Tooltip simple, muestra el valor en USD (puede mejorarse para mostrar también CUP) -->
					<Chart.Tooltip
						labelFormatter={(value) => `${value.toLocaleString('en-US', { style: 'currency', currency: 'USD' })}`}
					/>
				{/snippet}
			</BarChart>
		</Chart.Container>
	</Card.Content>
	<Card.Footer class="flex flex-col items-center justify-center gap-2">
		<div class="text-muted-foreground text-sm">{m.total_revenue()}</div>
		<!-- 👇 Mostrar ambas monedas -->
		<div class="text-2xl font-bold">
			{totalUSD.toLocaleString('en-US', { style: 'currency', currency: 'USD' })}
		</div>
		<div class="text-md font-semibold text-muted-foreground">
			{totalCUP.toLocaleString('en-US', { style: 'currency', currency: 'CUP' })}
			<span class="text-xs"> (1 USD = {currentRate} CUP)</span>
		</div>
	</Card.Footer>
</Card.Root>
