<script lang="ts">
	import { superForm } from 'sveltekit-superforms';
	import { zodClient } from 'sveltekit-superforms/adapters';
	import type { z } from 'zod';
	import { newMemberSchema, type NewMemberTypeSchema } from '$lib/schemas/new_member_schema';
	import type { Member } from '$lib/models/member';
	import { invoke } from '@tauri-apps/api/core';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';

	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import * as Form from '$lib/components/ui/form/index.js';
	import * as Card from '$lib/components/ui/card';
	import Label from '$lib/components/ui/label/label.svelte';
	import Input from '$lib/components/ui/input/input.svelte';
	import { getLocalTimeZone, parseDate, today, type DateValue } from '@internationalized/date';
	import { setHeader, setLoading } from '$lib/stores/state';
	import { onMount } from 'svelte';
	import type { ErrorResponse } from '$lib/models/error';
	import { m } from '$lib/paraglide/messages';
	import { translateErrorCode } from '$lib/utils';
	import Button from '$lib/components/ui/button/button.svelte';
	import DateField from '$lib/components/date-field/date-field.svelte';

	let newMember: null | Member = null;
	let showMembershipPrompt = false;
	const locale = m.locale_code() || 'bs-BA';

	// --- Photo handling ---
	let selectedPhoto: { bytes: Uint8Array; extension: string } | null = null;
	let photoPreviewUrl: string | null = null;

	function handlePhotoChange(event: Event) {
		console.log('FILE INPUT TRIGGERED');
		const input = event.target as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;

		if (!file.type.startsWith('image/')) {
			toast.error(m.only_images_allowed());
			return;
		}
		if (file.size > 5 * 1024 * 1024) {
			toast.error(m.file_too_large());
			return;
		}

		const reader = new FileReader();
		reader.onload = (e) => {
			const arrayBuffer = e.target?.result as ArrayBuffer;
			const bytes = new Uint8Array(arrayBuffer);
			const extension = file.name.split('.').pop() || 'jpg';
			selectedPhoto = { bytes, extension };
			photoPreviewUrl = URL.createObjectURL(file);
		};
		reader.readAsArrayBuffer(file);
	}

	function clearPhoto() {
		selectedPhoto = null;
		if (photoPreviewUrl) {
			URL.revokeObjectURL(photoPreviewUrl);
			photoPreviewUrl = null;
		}
		const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
		if (fileInput) fileInput.value = '';
	}
	// --- end photo handling ---

	const initialValues: z.infer<NewMemberTypeSchema> = {
		card_id: '',
		first_name: '',
		last_name: '',
		email: '',
		phone: '',
		date_of_birth: null
	};

	const form = superForm(initialValues, {
		validators: zodClient(newMemberSchema as any),
		syncFlashMessage: true,
		dataType: 'json',
		SPA: true,
		taintedMessage: null,
		resetForm: false,
		onUpdated({ form: currentForm }) {
			if (!currentForm.valid) console.log('Client errors:', currentForm.errors);
		}
	});

	const { form: formData, enhance } = form;

	function handleDateChange(newValue: DateValue | undefined) {
		$formData.date_of_birth = newValue ? newValue.toString() : null;
	}

	async function handleSubmit() {
		setLoading(true);
		newMember = null;
		try {
			const result = await form.validateForm();
			if (result.valid) {
				// 1. Create member
				const member: Member = await invoke('add_member', { payload: result.data });
				newMember = member;

				// 2. If photo selected, upload it
				if (selectedPhoto && member.id) {
					try {
						await invoke('save_member_photo', {
							memberId: member.id,
							photoBytes: Array.from(selectedPhoto.bytes),
							extension: selectedPhoto.extension
						});
						toast.success(m.photo_uploaded());
					} catch (photoError) {
						console.error('Photo upload failed:', photoError);
						toast.error(m.photo_upload_failed());
					}
				}

				showMembershipPrompt = true;
				toast.success(m.new_member_add_success());
			} else {
				toast.error(m.toast_error_invalid_data());
			}
		} catch (error) {
			showMembershipPrompt = false;
			const errorMessage = error as ErrorResponse;
			if (errorMessage?.error_code && errorMessage?.params) {
				toast.error(translateErrorCode(errorMessage.error_code, errorMessage.params));
			} else {
				toast.error(m.toast_error_new_member_fail());
			}
		} finally {
			setLoading(false);
		}
	}

	async function handleCancel() {
		await goto('/members');
	}

	async function handleCancelPrompt() {
		if (newMember) {
			await goto(`/members/${newMember.id}`);
		} else {
			await goto('/members');
		}
	}

	async function assignMembership() {
		if (newMember) {
			await goto(`/members/${newMember.id}/new-membership`);
		}
		showMembershipPrompt = false;
	}

	onMount(() => {
		setHeader({
			title: m.add_new_member(),
			showBackButton: true
		});
	});
	let fileInput: HTMLInputElement | null = null;
</script>

<div class="container mx-auto p-4 md:p-8 max-w-2xl">
	<Card.Root>
		<Card.Header>
			<Card.Title class="text-2xl">{m.add_new_member()}</Card.Title>
		</Card.Header>
		<Card.Content>
			<form use:enhance method="post" onsubmit={handleSubmit} class="space-y-6">
				<!-- Photo upload field -->
				<div class="space-y-2">
					<Label class="font-semibold">{m.photo()}</Label>
					<div class="flex items-center gap-4">
						{#if photoPreviewUrl}
							<img src={photoPreviewUrl} alt={m.member_photo()} class="w-20 h-20 rounded-full object-cover" />
						{:else}
							<div class="w-20 h-20 rounded-full bg-muted flex items-center justify-center">
								<span class="text-muted-foreground text-xs">{m.no_photo()}</span>
							</div>
						{/if}
						<div class="flex gap-2">
							<input
								type="file"
								accept="image/*"
								class="hidden"
								onchange={handlePhotoChange}
								bind:this={fileInput}
							/>

							<Button
								type="button"
								variant="outline"
								size="sm"
								onclick={() => fileInput?.click()}
							>
								{photoPreviewUrl ? m.change_photo() : m.upload_photo()}
							</Button>
							{#if photoPreviewUrl}
								<Button type="button" variant="destructive" size="sm" onclick={clearPhoto}>
									{m.remove_photo()}
								</Button>
							{/if}
						</div>
					</div>
					<p class="text-xs text-muted-foreground">{m.optional()}</p>
				</div>

				<Form.Field {form} name="first_name">
					<Form.Control>
						{#snippet children({ props })}
							<Form.Label class="font-semibold">{m.first_name()}</Form.Label>
							<Input {...props} type="text" bind:value={$formData.first_name} />
							<Form.FieldErrors />
						{/snippet}
					</Form.Control>
				</Form.Field>

				<Form.Field {form} name="last_name">
					<Form.Control>
						{#snippet children({ props })}
							<Form.Label class="font-semibold">{m.last_name()}</Form.Label>
							<Input {...props} type="text" bind:value={$formData.last_name} />
							<Form.FieldErrors />
						{/snippet}
					</Form.Control>
				</Form.Field>

				<Form.Field {form} name="email">
					<Form.Control>
						{#snippet children({ props })}
							<Form.Label class="font-semibold">{m.email()}</Form.Label>
							<Input {...props} type="email" bind:value={$formData.email} />
							<Form.Description class="text-xs">{m.optional()}</Form.Description>
							<Form.FieldErrors />
						{/snippet}
					</Form.Control>
				</Form.Field>

				<Form.Field {form} name="phone">
					<Form.Control>
						{#snippet children({ props })}
							<Form.Label class="font-semibold">{m.phone()}</Form.Label>
							<Input {...props} type="text" bind:value={$formData.phone} />
							<Form.Description class="text-xs">{m.optional()}</Form.Description>
							<Form.FieldErrors />
						{/snippet}
					</Form.Control>
				</Form.Field>

				<Form.Field {form} name="date_of_birth">
					<Form.Control>
						{#snippet children({ props })}
							<Form.Label class="font-semibold">{m.date_of_birth()}</Form.Label>
							<DateField
								value={$formData.date_of_birth ? parseDate($formData.date_of_birth) : undefined}
								{...props}
								onValueChange={handleDateChange}
								{locale}
								weekStartsOn={1}
							/>

							<Form.FieldErrors />
							<Form.Description class="text-xs">{m.optional()}</Form.Description>
						{/snippet}
					</Form.Control>
				</Form.Field>

				<Form.Field {form} name="card_id">
					<Form.Control>
						{#snippet children({ props })}
							<Form.Label class="font-semibold">{m.card_number()}</Form.Label>
							<Input
								{...props}
								type="text"
								bind:value={$formData.card_id}
								onkeydown={(e) => {
									if (e.key === 'Enter') {
										e.preventDefault();
									}
								}}
							/>
							<Form.Description class="text-xs">{m.use_scanner_or_enter()}</Form.Description>
							<Form.FieldErrors />
						{/snippet}
					</Form.Control>
				</Form.Field>

				<div class="flex gap-20 justify-around w-full">
					<Button variant="outline" onclick={handleCancel} class="flex-1"
						>{m['common.cancel']()}</Button
					>
					<Form.Button type="submit" class="flex-1">{m['common.confirm']()}</Form.Button>
				</div>
			</form>
		</Card.Content>
	</Card.Root>
	{#if newMember}
		<AlertDialog.Root bind:open={showMembershipPrompt}>
			<AlertDialog.Content>
				<AlertDialog.Header>
					<AlertDialog.Title>{m.membership_prompt()}</AlertDialog.Title>
					<AlertDialog.Description>
						{m['common.member']()}
						<b
							>{newMember.first_name}
							{newMember.last_name}</b
						>
						{m.membership_prompt_desc()}
					</AlertDialog.Description>
				</AlertDialog.Header>
				<AlertDialog.Footer>
					<AlertDialog.Cancel onclick={handleCancelPrompt}
						>{m.membership_prompt_cancel()}</AlertDialog.Cancel
					>
					<AlertDialog.Action onclick={assignMembership}
						>{m.membership_prompt_yes()}</AlertDialog.Action
					>
				</AlertDialog.Footer>
			</AlertDialog.Content>
		</AlertDialog.Root>
	{/if}
</div>