<script lang="ts">
	import { Calendar as CalendarPrimitive } from "bits-ui";
	import { cn, type WithoutChildrenOrChild } from "$lib/utils.js";
	import ChevronDownIcon from "@lucide/svelte/icons/chevron-down";

	let {
		ref = $bindable(null),
		class: className,
		value,
		onchange,
		...restProps
	}: WithoutChildrenOrChild<CalendarPrimitive.MonthSelectProps> = $props();
</script>

<CalendarPrimitive.MonthSelect bind:ref {...restProps}>
	{#snippet child({ props, monthItems, selectedMonthItem })}
		<div class="relative">
			<select
				{...props}
				{value}
				{onchange}
				class={cn(
					"appearance-none rounded-md border border-input bg-background px-3 py-1 pr-8 text-sm text-foreground shadow-xs",
					"focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-1",
					"disabled:cursor-not-allowed disabled:opacity-50",
					className
				)}
			>
				{#each monthItems as monthItem (monthItem.value)}
					<option
						value={monthItem.value}
						selected={value !== undefined
							? monthItem.value === value
							: monthItem.value === selectedMonthItem.value}
					>
						{monthItem.label}
					</option>
				{/each}
			</select>
			<ChevronDownIcon
				class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 size-4 text-muted-foreground"
			/>
		</div>
	{/snippet}
</CalendarPrimitive.MonthSelect>

<style>
	/* Forzar colores oscuros en las opciones del dropdown nativo */
	select option {
		background-color: #2d2d2d !important;
		color: #f0f0f0 !important;
	}
	select option:hover {
		background-color: #3d3d3d !important;
	}
	/* Indicar al navegador que prefiera tema oscuro para el select */
	select {
		color-scheme: dark;
	}
</style>