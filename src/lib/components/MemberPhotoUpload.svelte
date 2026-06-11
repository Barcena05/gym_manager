<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { m } from "$lib/paraglide/messages";
  import { toast } from "svelte-sonner";
  
  export let memberId: number | null = null;
  export let currentPhotoPath: string | null = null;
  export let onPhotoUpdate: (path: string | null) => void = () => {};
  
  let previewUrl: string | null = currentPhotoPath ? convertFileSrc(currentPhotoPath) : null;
  let uploading = false;
  
  async function handleFileUpload(event: Event) {
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
    
    uploading = true;
    
    try {
      const reader = new FileReader();
      reader.onload = async (e) => {
        const arrayBuffer = e.target?.result as ArrayBuffer;
        const bytes = new Uint8Array(arrayBuffer);
        const extension = file.name.split('.').pop() || 'jpg';
        
        const photoPath = await invoke<string>('save_member_photo', {
          memberId,
          photoBytes: Array.from(bytes),
          extension
        });
        
        previewUrl = convertFileSrc(photoPath);
        onPhotoUpdate(photoPath);
        toast.success(m.photo_uploaded());
      };
      reader.readAsArrayBuffer(file);
    } catch (error) {
      console.error('Failed to upload photo:', error);
      toast.error(m.photo_upload_failed());
    } finally {
      uploading = false;
      input.value = '';
    }
  }
  
  async function removePhoto() {
    if (!memberId) return;
    
    try {
      await invoke('delete_member_photo', { memberId });
      previewUrl = null;
      onPhotoUpdate(null);
      toast.success(m.photo_removed());
    } catch (error) {
      console.error('Failed to remove photo:', error);
      toast.error(m.photo_remove_failed());
    }
  }
</script>

<div class="flex flex-col items-center gap-4">
  {#if previewUrl}
    <img src={previewUrl} alt={m.member_photo()} class="w-32 h-32 rounded-full object-cover" />
  {:else}
    <div class="w-32 h-32 rounded-full bg-muted flex items-center justify-center">
      <span class="text-muted-foreground">{m.no_photo()}</span>
    </div>
  {/if}
  
  <div class="flex gap-2">
    <label class="cursor-pointer">
      <input type="file" accept="image/*" on:change={handleFileUpload} disabled={uploading} class="hidden" />
      <button type="button" class="text-sm" disabled={uploading}>
        {previewUrl ? m.change_photo() : m.upload_photo()}
      </button>
    </label>
    
    {#if previewUrl}
      <button type="button" class="text-sm text-destructive" on:click={removePhoto}>
        {m.remove_photo()}
      </button>
    {/if}
  </div>
</div>