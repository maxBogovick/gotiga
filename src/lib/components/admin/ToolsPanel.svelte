<script lang="ts">
    import { api } from '$lib/api';

    // ── Convert image ─────────────────────────────────────────────────────
    //
    // A one-off local conversion, not a media-library action: nothing here
    // gets saved to disk. The keeper picks a file, one or more target
    // formats, and an optional resolution cap, and gets a download for
    // each format — same Blob/createObjectURL pattern the CSV exports
    // elsewhere in the admin already use.

    let message = $state('');
    let error = $state('');

    function showMessage(text: string) {
        message = text;
        error = '';
        setTimeout(() => message = '', 3500);
    }

    function showError(text: string) {
        error = text;
        message = '';
    }

    let convertFile = $state<File | null>(null);
    let convertJpeg = $state(false);
    let convertPng = $state(true);
    let convertWebp = $state(false);
    // Empty = original size. Values mirror the caps the server already
    // enforces elsewhere (FRAME_MAX_PX/PREVIEW_PX/MEDIUM_PX/THUMB_PX in
    // handlers.rs) — the UI never invents a number the server doesn't
    // already treat as meaningful.
    let resolution = $state<'' | '1600' | '1800' | '900' | '420' | 'custom'>('');
    let customResolution = $state<number | null>(null);
    let converting = $state(false);

    function resolvedMaxDimension(): number | undefined {
        if (resolution === '') return undefined;
        if (resolution === 'custom') {
            return customResolution && customResolution > 0 ? customResolution : undefined;
        }
        return Number(resolution);
    }

    // Presets read off the same numbers the frame-upload pipeline itself
    // enforces (admin_upload_battle_frame_art caps at FRAME_MAX_PX = 1600)
    // and the format frames actually end up stored as — a one-click way to
    // pre-shrink a picture before it goes into a race's or a tier's frame.
    function applyCardFramePreset(format: 'webp' | 'png') {
        convertJpeg = false;
        convertPng = format === 'png';
        convertWebp = format === 'webp';
        resolution = '1600';
    }

    function applyOriginalPreset() {
        resolution = '';
    }

    function downloadBlob(blob: Blob, filename: string) {
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        document.body.appendChild(a);
        a.click();
        a.remove();
        URL.revokeObjectURL(url);
    }

    async function convertImage() {
        if (!convertFile) {
            showError('Choose a file to convert first.');
            return;
        }
        const formats: Array<'jpeg' | 'png' | 'webp'> = [
            ...(convertJpeg ? (['jpeg'] as const) : []),
            ...(convertPng ? (['png'] as const) : []),
            ...(convertWebp ? (['webp'] as const) : []),
        ];
        if (!formats.length) {
            showError('Pick at least one target format.');
            return;
        }
        const maxDimension = resolvedMaxDimension();
        const base = convertFile.name.replace(/\.[^.]+$/, '') || 'image';
        converting = true;
        try {
            for (const format of formats) {
                const blob = await api.adminConvertImage(convertFile, format, maxDimension);
                downloadBlob(blob, `${base}.${format === 'jpeg' ? 'jpg' : format}`);
            }
            showMessage(`Converted to ${formats.join(', ')}`);
        } catch (e) {
            showError(e instanceof Error ? e.message : String(e));
        } finally {
            converting = false;
        }
    }
</script>

<div class="h-full flex flex-col gap-4 p-6 overflow-y-auto">
    <h2 class="text-xl font-gothic">Tools</h2>

    {#if message}
        <div class="border border-emerald-700/30 bg-emerald-50 px-4 py-2 text-xs text-emerald-800">{message}</div>
    {/if}
    {#if error}
        <div class="border border-red-700/30 bg-red-50 px-4 py-2 text-xs text-red-800">{error}</div>
    {/if}

    <div class="border border-[#34251c]/15 px-4 py-3 max-w-2xl">
        <div class="text-xs uppercase tracking-wide text-[#34251c] mb-2">Convert image</div>

        <div class="flex flex-wrap items-center gap-2 mb-3">
            <span class="text-[10px] uppercase tracking-wide text-[#5f4636]">Presets:</span>
            <button onclick={() => applyCardFramePreset('webp')} class="btn-gothic text-[10px]">Card frame — WebP, ≤1600px</button>
            <button onclick={() => applyCardFramePreset('png')} class="btn-gothic text-[10px]">Card frame — PNG, ≤1600px</button>
            <button onclick={applyOriginalPreset} class="btn-gothic text-[10px]">Original size</button>
        </div>

        <div class="flex flex-wrap items-center gap-3">
            <input
                type="file"
                accept="image/*"
                onchange={(e) => (convertFile = e.currentTarget.files?.[0] ?? null)}
                class="text-xs"
            />
            <label class="flex items-center gap-1 text-xs text-[#5f4636]">
                <input type="checkbox" bind:checked={convertJpeg} /> JPEG
            </label>
            <label class="flex items-center gap-1 text-xs text-[#5f4636]">
                <input type="checkbox" bind:checked={convertPng} /> PNG
            </label>
            <label class="flex items-center gap-1 text-xs text-[#5f4636]">
                <input type="checkbox" bind:checked={convertWebp} /> WebP
            </label>
        </div>

        <div class="flex flex-wrap items-center gap-3 mt-3">
            <label class="flex items-center gap-2 text-xs text-[#5f4636]">
                Resolution
                <select bind:value={resolution} class="input-gothic text-xs py-1">
                    <option value="">Original size</option>
                    <option value="1600">≤ 1600 px (card frame)</option>
                    <option value="1800">≤ 1800 px (preview)</option>
                    <option value="900">≤ 900 px (medium)</option>
                    <option value="420">≤ 420 px (thumbnail)</option>
                    <option value="custom">Custom…</option>
                </select>
            </label>
            {#if resolution === 'custom'}
                <input
                    type="number"
                    min="1"
                    placeholder="max px"
                    bind:value={customResolution}
                    class="input-gothic text-xs py-1 w-24"
                />
            {/if}
            <button
                onclick={convertImage}
                disabled={converting || !convertFile}
                class="btn-gothic text-[10px] disabled:opacity-70"
            >{converting ? 'Converting…' : 'Convert'}</button>
        </div>
        <p class="text-[10px] text-[#5f4636] mt-2">
            The longer side is capped, keeping proportions — never stretched, never enlarged.
        </p>
    </div>
</div>
