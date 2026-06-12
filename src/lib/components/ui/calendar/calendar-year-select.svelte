<script lang="ts">
	import { Calendar as CalendarPrimitive } from "bits-ui";
	import { cn, type WithoutChildrenOrChild } from "$lib/utils.js";
	import ChevronDownIcon from "@lucide/svelte/icons/chevron-down";

	let {
		ref = $bindable(null),
		class: className,
		value,
		...restProps
	}: WithoutChildrenOrChild<CalendarPrimitive.YearSelectProps> = $props();
</script>

<CalendarPrimitive.YearSelect bind:ref {...restProps}>
	{#snippet child({ props, yearItems, selectedYearItem })}
		<div class="relative">
			<select
				{...props}
				{value}
				class={cn(
					"appearance-none rounded-md border border-input bg-background px-3 py-1 pr-8 text-sm text-foreground shadow-xs",
					"focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-1",
					"disabled:cursor-not-allowed disabled:opacity-50",
					className
				)}
			>
				{#each yearItems as yearItem (yearItem.value)}
					<option
						value={yearItem.value}
						selected={value !== undefined
							? yearItem.value === value
							: yearItem.value === selectedYearItem.value}
					>
						{yearItem.label}
					</option>
				{/each}
			</select>
			<ChevronDownIcon
				class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 size-4 text-muted-foreground"
			/>
		</div>
	{/snippet}
</CalendarPrimitive.YearSelect>

<style>
	select option {
		background-color: #2d2d2d !important;
		color: #f0f0f0 !important;
	}
	select option:hover {
		background-color: #3d3d3d !important;
	}
	select {
		color-scheme: dark;
	}
</style>