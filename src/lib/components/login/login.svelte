<script lang="ts">
    import { Input } from '$lib/components/ui/input/index.js';
    import * as Form from '$lib/components/ui/form/index.js';
    import * as Card from '$lib/components/ui/card/index.js';
    import { auth } from '$lib/stores/auth';
    import { superForm } from 'sveltekit-superforms';
    import { zodClient } from 'sveltekit-superforms/adapters';
    import { z } from 'zod';
    import { toast } from 'svelte-sonner';
    import { m } from '$lib/paraglide/messages';
    import { setLoading } from '$lib/stores/state';
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { appDataDir, join } from '@tauri-apps/api/path';
    import { convertFileSrc } from '@tauri-apps/api/core';

    let backgroundUrl = $state<string | null>(null);

    onMount(async () => {
        try {
            const settings = await invoke<any>('get_app_settings');
            if (settings.login_background_path) {
                const dataDir = await appDataDir();
                const fullPath = await join(dataDir, settings.login_background_path);
                backgroundUrl = convertFileSrc(fullPath);
            }
        } catch (e) {
            console.error(e);
        }
    });

    const formSchema = z.object({
        username: z.string().min(2).max(20),
        password: z.string().min(5).max(20)
    });

    const defaultValues = { username: '', password: '' };
    const form = superForm(defaultValues, {
        validators: zodClient(formSchema),
        dataType: 'json',
        SPA: true,
        taintedMessage: null
    });
    const { form: formData, enhance } = form;

    async function handleSubmit() {
        setLoading(true);
        const result = await form.validateForm();
        if (result.valid) {
            auth.clearError();
            const loginSuccess = await auth.login(result.data.username, result.data.password);
            if (loginSuccess) {
                toast.success(m['login.toast_success']());
            } else {
                toast.error(m['login.toast_fail']());
            }
        } else {
            toast.error(m['login.toast_validation']());
        }
        setLoading(false);
    }
</script>

<div class="relative min-h-screen w-full flex items-center justify-center bg-cover bg-center"
     style="background-image: url('{backgroundUrl || ''}'); background-color: #1a1a1a;">
    <!-- Overlay oscuro para legibilidad -->
    <div class="absolute inset-0 bg-black/50"></div>
    
    <Card.Root class="relative z-10 w-full max-w-md shadow-2xl">
        <Card.Header>
            <Card.Title class="text-2xl text-center">{m['login.title']()}</Card.Title>
            <Card.Description class="text-center">{m['login.description']()}</Card.Description>
        </Card.Header>
        <form use:enhance method="post" on:submit|preventDefault={handleSubmit}>
            <Card.Content class="grid gap-4">
                <Form.Field {form} name="username">
                    <Form.Control>
                        {#snippet children({ props })}
                            <Form.Label class="font-semibold">{m['login.username']()}</Form.Label>
                            <Input {...props} type="text" bind:value={$formData.username} />
                            <Form.FieldErrors />
                        {/snippet}
                    </Form.Control>
                </Form.Field>

                <Form.Field {form} name="password">
                    <Form.Control>
                        {#snippet children({ props })}
                            <Form.Label class="font-semibold">{m['login.password']()}</Form.Label>
                            <Input {...props} type="password" bind:value={$formData.password} />
                            <Form.FieldErrors />
                        {/snippet}
                    </Form.Control>
                </Form.Field>

                <Form.Button type="submit" class="w-full">{m['login.title']()}</Form.Button>
            </Card.Content>
        </form>
    </Card.Root>
</div>