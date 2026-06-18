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
  let showCamera = false;
  let videoElement: HTMLVideoElement | null = null;
  let stream: MediaStream | null = null;
  
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
  
  async function openCamera() {
    try {
      stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: 'user', width: { ideal: 1280 }, height: { ideal: 720 } }
      });
      showCamera = true;
      
      // Wait for DOM update then access video element
      setTimeout(() => {
        if (videoElement) {
          videoElement.srcObject = stream;
          videoElement.play();
        }
      }, 0);
    } catch (error) {
      console.error('Failed to access camera:', error);
      toast.error(m.camera_access_denied());
    }
  }
  
  function closeCamera() {
    if (stream) {
      stream.getTracks().forEach(track => track.stop());
      stream = null;
    }
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
      
      uploading = true;
      closeCamera();
      
      try {
        const arrayBuffer = await blob.arrayBuffer();
        const bytes = new Uint8Array(arrayBuffer);
        
        const photoPath = await invoke<string>('save_member_photo', {
          memberId,
          photoBytes: Array.from(bytes),
          extension: 'jpg'
        });
        
        previewUrl = convertFileSrc(photoPath);
        onPhotoUpdate(photoPath);
        toast.success(m.photo_captured());
      } catch (error) {
        console.error('Failed to save captured photo:', error);
        toast.error(m.photo_upload_failed());
      } finally {
        uploading = false;
      }
    }, 'image/jpeg', 0.9);
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
  
  <div class="flex gap-2 flex-wrap justify-center">
    <label class="cursor-pointer">
      <input type="file" accept="image/*" on:change={handleFileUpload} disabled={uploading} class="hidden" />
      <button type="button" class="text-sm" disabled={uploading}>
        {previewUrl ? m.change_photo() : m.upload_photo()}
      </button>
    </label>
    
    <button type="button" class="text-sm" on:click={openCamera} disabled={uploading || showCamera}>
      {m.take_photo_with_camera()}
    </button>
    
    {#if previewUrl}
      <button type="button" class="text-sm text-destructive" on:click={removePhoto}>
        {m.remove_photo()}
      </button>
    {/if}
  </div>

  {#if showCamera}
    <div class="fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-50">
      <div class="bg-background p-6 rounded-lg shadow-xl max-w-2xl w-full mx-4">
        <h2 class="text-lg font-semibold mb-4">{m.capture_photo()}</h2>
        <video bind:this={videoElement} class="w-full rounded-lg bg-black" autoplay muted playsinline></video>
        <div class="flex justify-center gap-4 mt-4">
          <button 
            type="button" 
            class="px-4 py-2 bg-primary text-primary-foreground rounded hover:bg-primary/90"
            on:click={capturePhoto}
            disabled={uploading}
          >
            {m.capture()}
          </button>
          <button 
            type="button" 
            class="px-4 py-2 bg-muted text-muted-foreground rounded hover:bg-muted/80"
            on:click={closeCamera}
            disabled={uploading}
          >
            {m.cancel()}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>