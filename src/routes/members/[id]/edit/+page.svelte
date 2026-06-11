<script lang="ts">
	import { superForm } from 'sveltekit-superforms';
	import { zodClient } from 'sveltekit-superforms/adapters';
	import type { z } from 'zod';
	import { invoke } from '@tauri-apps/api/core';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { page } from '$app/state';
	import * as Form from '$lib/components/ui/form/index.js';
	import * as Card from '$lib/components/ui/card';
	import Input from '$lib/components/ui/input/input.svelte';
	import { parseDate, today, type DateValue, getLocalTimeZone } from '@internationalized/date';
	import { onMount } from 'svelte';
	import { editMemberSchema, type EditMemberTypeSchema } from '$lib/schemas/edit_member_schema';
	import type { Member } from '$lib/models/member';
	import { setHeader, setLoading } from '$lib/stores/state';
	import Button from '$lib/components/ui/button/button.svelte';
	import type { ErrorResponse } from '$lib/models/error';
	import { m } from '$lib/paraglide/messages';
	import { translateErrorCode } from '$lib/utils';
	import DateField from '$lib/components/date-field/date-field.svelte';
	import { appDataDir, join } from '@tauri-apps/api/path';
	import { convertFileSrc } from '@tauri-apps/api/core';

	let error: string | null = $state(null);
	const memberId = $derived(page.params.id);
	const locale = m.locale_code() || 'bs-BA';
	
	let existingPhotoUrl = $state<string | null>(null);

	let selectedPhoto = $state<{
		bytes: Uint8Array;
		extension: string;
	} | null>(null);

	let photoPreviewUrl = $state<string | null>(null);

	async function fetchMember() {
		setLoading(true);
		error = null;

		try {
			const result = await invoke<Member>('get_member_by_id', {
				payload: { id: Number(memberId) }
			});

			if (result) {
				$formData.id = result.id;
				$formData.card_id = result.card_id ?? '';
				$formData.first_name = result.first_name;
				$formData.last_name = result.last_name;
				$formData.email = result.email;
				$formData.phone = result.phone;
				$formData.date_of_birth = result.date_of_birth ?? null;

				// 👇 FOTO EXISTENTE
				if (result.photo_path) {
					const dataDir = await appDataDir();
					const fullPath = await join(dataDir, result.photo_path);
					existingPhotoUrl = convertFileSrc(fullPath);
				}
			}
		} catch (e) {
			console.error(e);
			toast.error(m.failed_to_load_member());
		} finally {
			setLoading(false);
		}
	}
	function handlePhotoChange(event: Event) {
		console.log('Candelaaaaa')
		const input = event.target as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;

		if (!file.type.startsWith('image/')) return;

		const reader = new FileReader();

		reader.onload = (e) => {
			const buffer = e.target?.result as ArrayBuffer;

			selectedPhoto = {
				bytes: new Uint8Array(buffer),
				extension: file.name.split('.').pop() || 'jpg'
			};

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
	}
	const initialValues: z.infer<EditMemberTypeSchema> = {
		id: 0,
		card_id: '',
		first_name: '',
		last_name: '',
		email: '',
		phone: '',
		date_of_birth: null
	};

	const form = superForm(initialValues, {
		validators: zodClient(editMemberSchema as any),
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

	async function handleSubmit() {
		setLoading(true);
		try {
			const result = await form.validateForm();
			if (result.valid) {
				const member = await invoke<Member>('update_member', {
					payload: result.data
				});
				if (selectedPhoto) {
					await invoke('save_member_photo', {
						memberId: member.id,
						photoBytes: Array.from(selectedPhoto.bytes),
						extension: selectedPhoto.extension
					});
				}
				toast.success(m.member_update_success());
				goto(`/members/${member.id}`);
			} else {
				toast.error(m['toast_error_invalid_data']());
			}
		} catch (error) {
			console.log(error);
			const errorMessage = error as ErrorResponse;
			if (errorMessage?.error_code && errorMessage?.params) {
				toast.error(translateErrorCode(errorMessage.error_code, errorMessage.params));
			} else {
				toast.error(m.toast_error_update_member_fail());
			}

			return;
		} finally {
			setLoading(false);
		}
	}

	async function handleCancel() {
		await goto('/members');
	}

	let date_of_birth = $state<DateValue | undefined>();
	const todayDate = today(getLocalTimeZone());

	$effect(() => {
		date_of_birth = $formData.date_of_birth ? parseDate($formData.date_of_birth) : undefined;
	});

	function handleDobChange(newValue: DateValue | undefined) {
		$formData.date_of_birth = newValue ? newValue.toString() : null;
	}

	onMount(async () => {
		setHeader({
			title: m.edit_member(),
			showBackButton: true
		});
		if (memberId) {
			fetchMember();
		}
	});
	let fileInput: HTMLInputElement;
</script>

<div class="container mx-auto p-4 md:p-8 max-w-2xl">
	<Card.Root>
		<Card.Header>
			<Card.Title class="text-2xl">{m['common.member']()}</Card.Title>
		</Card.Header>
		<Card.Content>			
			<form use:enhance method="post" onsubmit={handleSubmit} class="space-y-10">
				<div class="space-y-2">
				<div class="font-semibold">Photo</div>

				<div class="flex items-center gap-4">
					{#if photoPreviewUrl}
						<img src={photoPreviewUrl} alt={m.member_photo()} class="w-20 h-20 rounded-full object-cover" />
					{:else if existingPhotoUrl}
						<img src={existingPhotoUrl} alt={m.member_photo()} class="w-20 h-20 rounded-full object-cover" />
					{:else}
						<div class="w-20 h-20 rounded-full bg-muted flex items-center justify-center">
							<span>No photo</span>
						</div>
					{/if}

					<input
						bind:this={fileInput}
						type="file"
						accept="image/*"
						hidden
						onchange={handlePhotoChange}
					/>

					<Button
						type="button"
						variant="outline"
						size="sm"
						onclick={() => fileInput?.click()}
					>
						Change photo
					</Button>

					{#if selectedPhoto}
						<Button type="button" variant="destructive" size="sm" onclick={clearPhoto}>
							Remove
						</Button>
					{/if}
				</div>
			</div>
				<div class="space-y-6">
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
									{...props}
									value={date_of_birth}
									onValueChange={handleDobChange}
									locale={locale}
									maxValue={todayDate}
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
								<Input {...props} type="text" bind:value={$formData.card_id} />
								<Form.Description class="text-xs">{m.use_scanner_or_enter()}</Form.Description>
								<Form.FieldErrors />
							{/snippet}
						</Form.Control>
					</Form.Field>

					<div class="flex gap-20 justify-around">
						<Button variant="outline" onclick={handleCancel} class="flex-1">{m['common.cancel']()}</Button>
						<Form.Button type="submit" class="flex-1">{m['common.save']()}</Form.Button>
					</div>
				</div>
			</form>
		</Card.Content>
	</Card.Root>
</div>
