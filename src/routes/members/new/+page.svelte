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
	import SwitchCamera from 'lucide-svelte/icons/switch-camera';

	let newMember: null | Member = $state(null);
	let showMembershipPrompt = $state(false);
	const locale = $derived(m.locale_code() || 'bs-BA');

	// --- Photo handling ---
	let selectedPhoto: { bytes: Uint8Array; extension: string } | null = $state(null);
	let photoPreviewUrl: string | null = $state(null);

	// --- Camera functionality ---
	type CameraFacingMode = 'user' | 'environment';
	let showCamera = $state(false);
	let videoElement: HTMLVideoElement | null = $state(null);
	let stream: MediaStream | null = $state(null);
	let facingMode = $state<CameraFacingMode>('user');
	let switchingCamera = $state(false);
	let cameraRequestId = 0;

	function stopStream() {
		if (stream) {
			stream.getTracks().forEach((track) => track.stop());
			stream = null;
		}
	}

	async function startCamera(mode: CameraFacingMode, notifyOnError = true): Promise<boolean> {
		const requestId = ++cameraRequestId;
		stopStream();

		try {
			const nextStream = await navigator.mediaDevices.getUserMedia({
				video: { facingMode: mode, width: { ideal: 1280 }, height: { ideal: 720 } }
			});

			if (requestId !== cameraRequestId) {
				nextStream.getTracks().forEach((track) => track.stop());
				return false;
			}

			stream = nextStream;
			facingMode = mode;
			showCamera = true;

			setTimeout(() => {
				if (requestId !== cameraRequestId) return;
				if (videoElement) {
					videoElement.srcObject = nextStream;
					videoElement.play();
				}
			}, 0);

			return true;
		} catch (error) {
			console.error(`Failed to access ${mode} camera:`, error);
			if (requestId === cameraRequestId && notifyOnError) {
				toast.error(m.camera_access_denied());
			}
			return false;
		}
	}

	async function openCamera() {
		await startCamera(facingMode);
	}

	async function switchCamera() {
		if (switchingCamera) return;

		const previousMode = facingMode;
		const nextMode: CameraFacingMode = previousMode === 'user' ? 'environment' : 'user';
		switchingCamera = true;

		try {
			const switched = await startCamera(nextMode, false);
			if (!showCamera || switched) return;

			const restored = await startCamera(previousMode, false);
			if (!showCamera) return;

			if (restored) {
				toast.error(m.camera_switch_failed());
			} else {
				closeCamera();
				toast.error(m.camera_access_denied());
			}
		} finally {
			switchingCamera = false;
		}
	}

	function closeCamera() {
		cameraRequestId += 1;
		stopStream();
		showCamera = false;
	}
	
	async function capturePhoto() {
		if (!videoElement) return;
		
		const canvas = document.createElement('canvas');
		canvas.width = videoElement.videoWidth;
		canvas.height = videoElement.videoHeight;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;
		
		ctx.drawImage(videoElement, 0, 0, canvas.width, canvas.height);
		
		canvas.toBlob(async (blob) => {
			if (!blob) {
				toast.error(m.photo_capture_failed());
				return;
			}
			
			const arrayBuffer = await blob.arrayBuffer();
			const bytes = new Uint8Array(arrayBuffer);
			
			selectedPhoto = {
				bytes: bytes,
				extension: 'jpg'
			};
			
			const file = new File([blob], 'captured.jpg', { type: 'image/jpeg' });
			photoPreviewUrl = URL.createObjectURL(file);
			
			closeCamera();
			toast.success(m.photo_captured());
		}, 'image/jpeg', 0.9);
	}
	// --- end camera functionality ---

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
							
							<Button
								type="button"
								variant="outline"
								size="sm"
								onclick={openCamera}
								disabled={showCamera}
							>
								{m.take_photo_with_camera()}
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
						<p class="mt-2 font-mono text-sm">ID: {newMember.card_id ?? ''}</p>
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

<!-- Camera Modal -->
{#if showCamera}
	<div class="fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-50">
		<div class="bg-background p-6 rounded-lg shadow-xl max-w-2xl w-full mx-4">
			<div class="mb-4 flex items-center justify-between gap-3">
				<h2 class="text-lg font-semibold">{m.capture_photo()}</h2>
				<button
					type="button"
					class="inline-flex items-center gap-2 rounded-md border border-input bg-background px-3 py-2 text-sm font-medium shadow-sm transition-colors hover:bg-accent hover:text-accent-foreground disabled:pointer-events-none disabled:opacity-50"
					onclick={switchCamera}
					disabled={switchingCamera}
					aria-label={m.switch_camera()}
					title={m.switch_camera()}
				>
					<SwitchCamera class="h-4 w-4" />
					<span class="hidden sm:inline">{m.switch_camera()}</span>
				</button>
			</div>
			<video bind:this={videoElement} class="w-full rounded-lg bg-black" autoplay muted playsinline></video>
			<div class="flex justify-center gap-4 mt-4">
				<button 
					type="button" 
					class="px-4 py-2 bg-primary text-primary-foreground rounded hover:bg-primary/90 disabled:pointer-events-none disabled:opacity-50"
					onclick={capturePhoto}
					disabled={switchingCamera}
				>
					{m.capture()}
				</button>
				<button 
					type="button" 
					class="px-4 py-2 bg-muted text-muted-foreground rounded hover:bg-muted/80"
					onclick={closeCamera}
				>
					{m.cancel()}
				</button>
			</div>
		</div>
	</div>
{/if}
