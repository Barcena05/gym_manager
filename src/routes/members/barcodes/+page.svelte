<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { Printer, ArrowLeft } from 'lucide-svelte';
	import { goto } from '$app/navigation';
	import Button from '$lib/components/ui/button/button.svelte';
	import BarcodeLabel from '$lib/components/BarcodeLabel.svelte';
	import type { MemberInfo } from '$lib/models/member_with_membership';
	import type { QueryResponse } from '$lib/models/table-state';
	import { setHeader, setLoading } from '$lib/stores/state';
	import { m } from '$lib/paraglide/messages';
	import { buildBarcodePdfBase64, isAndroid, savePdfAndOpen } from '$lib/barcode-pdf';
	import { printElementById } from '$lib/print';

	let members = $state<MemberInfo[]>([]);
	let loading = $state(true);
	let generatingPdf = $state(false);

	async function loadMembers() {
		loading = true;
		setLoading(true);
		try {
			const response = await invoke<QueryResponse<MemberInfo>>(
				'get_members_with_memberships_paginated',
				{
					payload: {
						page: 1,
						per_page: 1000,
						search_string: '',
						filter_fields: []
					}
				}
			);
			members = (response.data || []).filter((member) => !!member.card_id);
		} catch (e) {
			console.error('Error loading members for barcodes:', e);
			toast.error(m.failed_to_fetch_members());
		} finally {
			loading = false;
			setLoading(false);
		}
	}

	async function printBarcodes() {
		if (!isAndroid()) {
			printElementById('barcode-print-area');
			return;
		}

		generatingPdf = true;
		try {
			const base64 = await buildBarcodePdfBase64(
				members.map((member) => ({
					value: member.card_id ?? '',
					name: `${member.first_name} ${member.last_name}`.trim()
				}))
			);
			await savePdfAndOpen(base64);
		} catch (e) {
			console.error('Error generando/abriendo el PDF:', e);
			toast.error(`No se pudo generar el PDF: ${e instanceof Error ? e.message : String(e)}`);
		} finally {
			generatingPdf = false;
		}
	}

	onMount(() => {
		setHeader({
			title: m['barcode.print_all'](),
			showBackButton: true
		});
		loadMembers();
	});
</script>

<div class="container mx-auto p-4 md:p-8 max-w-7xl">
	<div class="flex flex-wrap items-center justify-between gap-3 mb-6 print:hidden">
		<div class="flex items-center gap-3">
			<Button variant="outline" size="icon" onclick={() => goto('/members')}>
				<ArrowLeft class="h-4 w-4" />
			</Button>
			<h1 class="text-xl font-semibold">{m['barcode.print_all']()}</h1>
		</div>
		<div class="flex items-center gap-3">
			<span class="text-sm text-muted-foreground">
				{members.length} {m['common.members']()}
			</span>
			<Button onclick={printBarcodes} disabled={loading || members.length === 0 || generatingPdf}>
				<Printer class="h-4 w-4 mr-2" />
				{generatingPdf ? 'Generando PDF...' : m['barcode.print']()}
			</Button>
		</div>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-16">
			<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
		</div>
	{:else if members.length === 0}
		<div class="text-center py-16 text-muted-foreground">{m.no_results()}</div>
	{:else}
		<div
			id="barcode-print-area"
			class="barcode-grid grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4 print:grid-cols-3 print:gap-2"
		>
			{#each members as member (member.id)}
				<BarcodeLabel
					value={member.card_id ?? ''}
					name={`${member.first_name} ${member.last_name}`}
				/>
			{/each}
		</div>
	{/if}
</div>

