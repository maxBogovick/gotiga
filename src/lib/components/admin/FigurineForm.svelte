<script lang="ts">
  /**
   * The registry's work editor: everything to the right of the figurine list.
   *
   * Owns its own media uploads, keyhole/parallax knobs and the two side-channel
   * texts (the search caption and the Pinterest description) — those have their
   * own endpoints and are never touched by an ordinary figurine save.
   *
   * `figurine` is the page's selected work, passed straight through: it is a
   * $state proxy, so field writes here land on the page's object. Switching to
   * another work while an upload is in flight is blocked by the page (it watches
   * `busy`); the id checks around every await are the second line of defence.
   */
  import { untrack } from 'svelte';
  import { api } from '$lib/api';
  import { fillTemplate } from '$lib/gazette';
  import { generatePinterestDescription } from '$lib/pinterest-description';
  import { formatFigurineAlt, altLabelsFrom, siblingPosition } from '$lib/figurine-alt';
  import type { Figurine, FigurineListItem, ShowingRoom } from '$lib/types/api';
  import { fade, slide } from 'svelte/transition';
  import { t } from '$lib/i18n';
  import { themeConfig } from '$lib/stores/theme.svelte';
  import { resolveWindow, isShowingOpen, minutesToClock, clockToMinutes } from '$lib/showing-window';
  import KeyholeVeil from '$lib/components/KeyholeVeil.svelte';
  import SealedDoor from '$lib/components/SealedDoor.svelte';
  import FigurineShowingsEditor from '$lib/components/admin/FigurineShowingsEditor.svelte';
  import DisplayConfigEditor from '$lib/components/admin/DisplayConfigEditor.svelte';

  let {
    figurine,
    figurines,
    showingRooms,
    unsaved = false,
    deleting = false,
    busy = $bindable(false),
    onSaved,
    onDelete,
    onCancel,
    onMessage,
  }: {
    figurine: Figurine;
    /** The whole registry — powers the material/technique/dimensions suggestions. */
    figurines: FigurineListItem[];
    showingRooms: ShowingRoom[];
    unsaved: boolean;
    deleting: boolean;
    /** True while a photo/video/audio/folder upload is in flight (page reads it). */
    busy: boolean;
    onSaved: () => Promise<void>;
    onDelete: () => void;
    onCancel: () => void;
    onMessage: (text: string, type?: string) => void;
  } = $props();

  let isSaving = $state(false);
  let showingsEditor = $state<FigurineShowingsEditor | null>(null);
  let activeFormTab = $state<'media' | 'text' | 'object' | 'passport' | 'vitrina'>('media');
  let selectedImageIdx = $state<number | null>(null);
  let uploadingVideo = $state(false);
  let uploadingImage = $state(false);
  let uploadingAudio = $state(false);
  let externalVideoUrl = $state('');
  let folderUploadProgress = $state<{ done: number; total: number } | null>(null);

  $effect(() => {
    busy = uploadingVideo || uploadingAudio || uploadingImage || folderUploadProgress !== null;
  });

  let materialSuggestions = $derived(
    [...new Set(figurines.map(f => f.material).filter((v): v is string => !!v))].sort()
  );
  let techniqueSuggestions = $derived(
    [...new Set(figurines.map(f => f.technique).filter((v): v is string => !!v))].sort()
  );
  let dimensionsSuggestions = $derived(
    [...new Set(figurines.map(f => f.dimensions).filter((v): v is string => !!v))].sort()
  );

  // See a window "as a guest would" at any moment — nothing is saved.
  let previewAt = $state<Date>(new Date());
  function toLocalInput(d: Date): string {
    const p = (n: number) => String(n).padStart(2, '0');
    return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
  }
  let previewFigWindow = $derived(
    resolveWindow(
      { openFromMin: figurine.openFromMin, openUntilMin: figurine.openUntilMin, showingRoomId: figurine.showingRoomId },
      showingRooms
    )
  );
  let previewFigOpen = $derived(isShowingOpen(previewFigWindow, previewAt));

    // Backstage search caption ("Хранитель") — edited through its own endpoint,
    // NOT the main figurine save, so ordinary saves never touch it. Loaded lazily
    // when a work opens.
    let captionText = $state('');
    let captionLoading = $state(false);
    let captionSaving = $state(false);
    // False for a work that exists only in this form (new / duplicated): the
    // caption and Pinterest endpoints key off a saved row, so they wait for one.
    let figurineExists = $derived(figurines.some((f) => f.id === figurine.id));

    let gzTitle = $state('');
    let gzSummary = $state('');
    let gzBusy = $state(false);
    let gzForId = $state('');

    $effect(() => {
        const id = figurine.id;
        const name = figurine.name;
        if (id === gzForId) return;
        gzForId = id;
        const fill = fillTemplate('arrival', name);
        gzTitle = fill.titleEn;
        gzSummary = fill.dekEn;
    });

    function faceUrl(): string | null {
        return figurine.images.find((img) => img.imageType === 'face')?.url
            ?? figurine.images[0]?.url
            ?? null;
    }

    async function publishGazetteNote() {
        if (!figurineExists) return;
        if (!gzTitle.trim()) {
            onMessage($t('adminGazetteNeedTitle'), 'error');
            return;
        }
        gzBusy = true;
        try {
            const handle = figurine.slug ?? figurine.id;
            await api.adminSaveGazetteLeaf({
                kind: 'arrival',
                status: 'published',
                titleEn: gzTitle.trim(),
                titleRu: gzTitle.trim(),
                dekEn: gzSummary.trim() || null,
                dekRu: gzSummary.trim() || null,
                figurineId: figurine.id,
                href: `/figurines/${handle}`,
                imageUrl: faceUrl(),
            });
            onMessage($t('adminGazetteSaved'), 'success');
        } catch (e: unknown) {
            onMessage($t('adminMsgError') + (e instanceof Error ? e.message : String(e)), 'error');
        } finally {
            gzBusy = false;
        }
    }

    async function saveCaption() {
        captionSaving = true;
        try {
            await api.setFigurineCaption(figurine.id, captionText);
            onMessage($t('adminCaptionSaved'), 'success');
        } catch (e: unknown) {
            onMessage($t('adminMsgError') + (e instanceof Error ? e.message : String(e)), 'error');
        } finally {
            captionSaving = false;
        }
    }

    // Pinterest SEO description ("Подключить RSS-канал" target) — same shape as
    // the search caption above: its own endpoint, loaded lazily, never touched
    // by the ordinary figurine save.
    let pinterestDescText = $state('');
    let pinterestDescLoading = $state(false);
    let pinterestDescSaving = $state(false);

    function generatePinterestDesc() {
        pinterestDescText = generatePinterestDescription(figurine);
    }

    async function savePinterestDesc() {
        pinterestDescSaving = true;
        try {
            await api.setFigurinePinterestDescription(figurine.id, pinterestDescText);
            onMessage($t('adminPinterestDescSaved'), 'success');
        } catch (e: unknown) {
            onMessage($t('adminMsgError') + (e instanceof Error ? e.message : String(e)), 'error');
        } finally {
            pinterestDescSaving = false;
        }
    }

    function resolveUrl(path: string | null): string {
        if (!path) return '';
        if (path.startsWith('http')) return path;
        if (path.startsWith('/static/')) {
            // Web-uploaded relative path — prepend server origin
            if (typeof localStorage !== 'undefined') {
                const serverUrl = localStorage.getItem('gotiga_server_url') ?? '';
                return serverUrl ? `${serverUrl}${path}` : path;
            }
        }
        return path;
    }

    function loadImageAspect(url: string): Promise<number | null> {
        if (!url) return Promise.resolve(null);
        return new Promise((resolve) => {
            const img = new Image();
            img.onload = () => {
                resolve(img.naturalWidth && img.naturalHeight
                    ? img.naturalWidth / img.naturalHeight
                    : null);
            };
            img.onerror = () => resolve(null);
            img.src = url;
        });
    }

    async function confirmDepthAspectMatches(imageUrl: string, depthUrl: string): Promise<boolean> {
        const [imageAspect, depthAspect] = await Promise.all([
            loadImageAspect(resolveUrl(imageUrl)),
            loadImageAspect(resolveUrl(depthUrl)),
        ]);
        if (!imageAspect || !depthAspect) return true;
        const drift = Math.abs(imageAspect - depthAspect) / imageAspect;
        return drift <= 0.03 || confirm($t('adminMediaDepthAspectWarning'));
    }


    // The figurine's window mode: '' = always open, 'custom' = own hours, else a room id.
    function figWindowMode(f: Figurine | null): string {
        if (!f) return '';
        if (f.showingRoomId) return f.showingRoomId;
        if (f.openFromMin != null && f.openUntilMin != null) return 'custom';
        return '';
    }

    // Room and custom hours are mutually exclusive: switching mode clears the other.
    function setFigWindowMode(value: string) {
        if (value === '') {
            figurine.showingRoomId = null;
            figurine.openFromMin = null;
            figurine.openUntilMin = null;
        } else if (value === 'custom') {
            figurine.showingRoomId = null;
            if (figurine.openFromMin == null) figurine.openFromMin = 0;
            if (figurine.openUntilMin == null) figurine.openUntilMin = 4 * 60;
        } else {
            figurine.showingRoomId = value;
            figurine.openFromMin = null;
            figurine.openUntilMin = null;
        }
    }


    function pickFileWeb(type: 'images' | 'videos' | 'audio'): Promise<File> {
        return new Promise((resolve, reject) => {
            const input = document.createElement('input');
            input.type = 'file';
            if (type === 'images') input.accept = 'image/jpeg,image/png,image/webp';
            else if (type === 'videos') input.accept = 'video/mp4,video/webm';
            else input.accept = 'audio/mpeg,audio/wav,audio/ogg';
            input.onchange = () => {
                const file = input.files?.[0];
                if (file) resolve(file);
                else reject(new Error('no file'));
            };
            input.click();
        });
    }

    // The picker rejects with 'no file' when the user cancels, which callers swallow.
    async function pickMediaSource(type: 'images' | 'videos' | 'audio'): Promise<File | null> {
        return await pickFileWeb(type);
    }

    async function handlePickFile(type: 'images' | 'videos' | 'audio', stepIndex?: number) {
        // Captured before the awaits below (file picker, then upload) so the result can be
        // checked against whatever figurine points to once they resolve — see
        // uploadBusy()'s comment for why that check matters.
        const targetId = figurine.id;

        // The picker sits OUTSIDE the busy window on purpose: while its dialog is open
        // nothing is in flight, so raising the flag here would tell an admin who opened it,
        // changed their mind and clicked another figurine to "wait for the upload to finish"
        // when there is no upload. Switching away during the dialog is allowed — the file
        // that comes back is simply dropped, since it was picked for a figurine that is no
        // longer open and attaching it to the new one would be a guess.
        const fileOrPath = await pickMediaSource(type).catch((e: unknown) => {
            const msg = e instanceof Error ? e.message : String(e);
            if (msg !== 'no file') onMessage($t('adminMsgError') + msg, 'error');
            return null;
        });
        if (fileOrPath === null) return;

        if (!figurine || figurine.id !== targetId) {
            onMessage($t('adminMsgUploadTargetChanged'), 'error');
            return;
        }

        // From here on there IS a transfer to protect: the flag blocks a figurine switch
        // until it lands (editFigurine/createNew/duplicateFigurine consult uploadBusy).
        if (type === 'videos') uploadingVideo = true;
        if (type === 'audio') uploadingAudio = true;
        if (type === 'images') uploadingImage = true;
        try {
            const imported = await api.importMediaWithVariants(
                fileOrPath,
                type === 'videos' ? 'videos' : type === 'audio' ? 'audio' : 'images',
                type === 'images' ? figurine.name : undefined
            );
            const localUrl = imported.url;

            if (!figurine || figurine.id !== targetId) {
                onMessage($t('adminMsgUploadTargetChanged'), 'error');
                return;
            }

            if (type === 'videos') {
                figurine.videoUrl = localUrl;
            } else if (type === 'audio') {
                figurine.ambiencePath = localUrl;
            } else if (typeof stepIndex === 'number') {
                figurine.processSteps[stepIndex].imageUrl = localUrl;
            } else {
                const variants = deriveImageVariants(localUrl);
                figurine.images = [...figurine.images, {
                    id: crypto.randomUUID(),
                    imageType: 'full',
                    url: localUrl,
                    originalUrl: imported.originalUrl ?? variants.originalUrl,
                    thumbUrl: imported.thumbUrl ?? variants.thumbUrl,
                    altText: '',
                    depthUrl: null,
                    parallaxIntensity: null,
                    focalX: null,
                    focalY: null,
                    revealRadius: null,
                    darkness: null
                }];
            }
            onMessage($t('adminMsgFileUploaded'), 'success');
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            if (msg !== 'no file') onMessage($t('adminMsgError') + msg, 'error');
        } finally {
            if (type === 'videos') uploadingVideo = false;
            if (type === 'audio') uploadingAudio = false;
            if (type === 'images') uploadingImage = false;
        }
    }

    async function handleFolderUpload() {
        // See uploadBusy()'s comment: captured once, checked before every write into
        // figurine.images so a figurine switch mid-batch (blocked at the entry
        // points by folderUploadProgress, but checked again here too) can never attach
        // a photo from this batch to a different figurine.
        const targetId = figurine.id;
        // Captured alongside targetId — figurine may go null mid-batch (see
        // above), so reading .name off it later in the loop isn't safe.
        const targetName = figurine.name;
        // Native folder picker via webkitdirectory.
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = 'image/jpeg,image/png,image/webp';
        input.multiple = true;
        (input as HTMLInputElement & { webkitdirectory: boolean }).webkitdirectory = true;
        input.onchange = async () => {
            const files = Array.from(input.files ?? [])
                .filter(f => /\.(jpe?g|png|webp)$/i.test(f.name))
                .sort((a, b) => a.name.localeCompare(b.name));
            if (files.length === 0) return;
            folderUploadProgress = { done: 0, total: files.length };
            for (const file of files) {
                try {
                    const imported = await api.importMediaWithVariants(file, 'images', targetName);
                    if (!figurine || figurine.id !== targetId) {
                        onMessage($t('adminMsgUploadTargetChanged'), 'error');
                        break;
                    }
                    const variants = deriveImageVariants(imported.url);
                    figurine.images = [...figurine.images, {
                        id: crypto.randomUUID(),
                        imageType: 'full',
                        url: imported.url,
                        originalUrl: imported.originalUrl ?? variants.originalUrl,
                        thumbUrl: imported.thumbUrl ?? variants.thumbUrl,
                        altText: '',
                        depthUrl: null,
                        parallaxIntensity: null,
                        focalX: null,
                        focalY: null,
                        revealRadius: null,
                        darkness: null
                    }];
                } catch (e) {
                    onMessage($t('adminMsgError') + String(e), 'error');
                }
                folderUploadProgress = { done: (folderUploadProgress?.done ?? 0) + 1, total: files.length };
            }
            folderUploadProgress = null;
            onMessage($t('adminMsgFileUploaded'), 'success');
        };
        input.click();
    }

    // Attach a precomputed depth map to a single image (LivingDaguerreotype 2.5D
    // parallax). It's just a grayscale image upload — the offline batch produces
    // higher-fidelity maps, this is the manual path. NULL falls back to luminance.
    async function handlePickDepth(imgIdx: number) {
        const targetId = figurine.id;

        // Same shape as handlePickFile: the picker's dialog is not a transfer, so it must
        // not raise the busy flag and block a figurine switch behind it.
        const fileOrPath = await pickMediaSource('images').catch((e: unknown) => {
            const msg = e instanceof Error ? e.message : String(e);
            if (msg !== 'no file') onMessage($t('adminMsgError') + msg, 'error');
            return null;
        });
        if (fileOrPath === null) return;

        if (!figurine || figurine.id !== targetId) {
            onMessage($t('adminMsgUploadTargetChanged'), 'error');
            return;
        }

        uploadingImage = true;
        try {
            const imported = await api.importMediaWithVariants(fileOrPath, 'images');
            if (!figurine || figurine.id !== targetId) {
                onMessage($t('adminMsgUploadTargetChanged'), 'error');
                return;
            }
            const targetImage = figurine.images[imgIdx];
            if (targetImage && !(await confirmDepthAspectMatches(targetImage.url, imported.url))) {
                onMessage($t('adminMediaDepthCancelled'), 'info');
                return;
            }
            if (!figurine || figurine.id !== targetId) {
                onMessage($t('adminMsgUploadTargetChanged'), 'error');
                return;
            }
            figurine.images[imgIdx].depthUrl = imported.url;
            figurine.images = [...figurine.images];
            onMessage($t('adminMediaDepthUploaded'), 'success');
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            if (msg !== 'no file') onMessage($t('adminMsgError') + msg, 'error');
        } finally {
            uploadingImage = false;
        }
    }

    function clearDepth(imgIdx: number) {
        figurine.images[imgIdx].depthUrl = null;
        figurine.images = [...figurine.images];
    }

    let generatingDepth = $state(false);

    // Generate depth maps for every image of the current figurine via the Rust
    // API (Depth-Anything on CPU). Requires the figurine to already exist server
    // side; then refresh each image's depthUrl so the badge/preview update.
    async function generateDepth() {
        if (unsaved) {
            onMessage($t('adminMediaDepthGenSaveFirst'), 'info');
            return;
        }
        generatingDepth = true;
        try {
            const res = await api.generateFigurineDepth(figurine.id);
            // Pull fresh depthUrls (the API just wrote them) without clobbering
            // any in-form edits: merge by image id.
            const fresh = await api.getFigurine(figurine.id);
            const byId = new Map((fresh?.images ?? []).map(i => [i.id, i.depthUrl ?? null]));
            figurine.images = figurine.images.map(img => ({
                ...img,
                depthUrl: byId.get(img.id) ?? img.depthUrl ?? null,
            }));
            onMessage(`${$t('adminMediaDepthGenDone')}: ${res.generated}/${res.results.length}`, 'success');
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            onMessage($t('adminMsgError') + msg, 'error');
        } finally {
            generatingDepth = false;
        }
    }

    // === BULK OPS (ADMIN — apply across every figurine at once) ===


    function setParallaxIntensity(imgIdx: number, value: string) {
        const parsed = Number(value);
        figurine.images[imgIdx].parallaxIntensity = Number.isFinite(parsed)
            ? Math.max(0, Math.min(1, parsed))
            : null;
        figurine.images = [...figurine.images];
    }

    function resetParallaxIntensity(imgIdx: number) {
        figurine.images[imgIdx].parallaxIntensity = null;
        figurine.images = [...figurine.images];
    }

    function parallaxValue(value: number | null | undefined): number {
        if (typeof value !== 'number' || !Number.isFinite(value)) return 0.6;
        return Math.max(0, Math.min(1, value));
    }

    // "Keyhole" reveal — the focal fragment shown on the archive/home card while
    // the work is still sealed (unseen). Frame-relative 0..1, edited over a 4/3
    // `contain` preview that mirrors the live card exactly.
    function setFocalPoint(imgIdx: number, x: number, y: number) {
        figurine.images[imgIdx].focalX = Math.round(x * 1000) / 1000;
        figurine.images[imgIdx].focalY = Math.round(y * 1000) / 1000;
        figurine.images = [...figurine.images];
    }

    function setRevealRadius(imgIdx: number, value: string) {
        const parsed = Number(value);
        figurine.images[imgIdx].revealRadius = Number.isFinite(parsed)
            ? Math.max(0.08, Math.min(1, parsed))
            : null;
        figurine.images = [...figurine.images];
    }

    function resetReveal(imgIdx: number) {
        figurine.images[imgIdx].focalX = null;
        figurine.images[imgIdx].focalY = null;
        figurine.images[imgIdx].revealRadius = null;
        figurine.images[imgIdx].darkness = null;
        figurine.images = [...figurine.images];
    }

    function revealRadiusValue(value: number | null | undefined): number {
        if (typeof value !== 'number' || !Number.isFinite(value)) return 0.3;
        return Math.max(0.08, Math.min(1, value));
    }

    // Per-image darkness override. Empty/non-finite → null = inherit the global
    // keyhole darkness (theme setting). Mirrors the renderer's 0.88 default.
    function setDarkness(imgIdx: number, value: string) {
        const parsed = Number(value);
        figurine.images[imgIdx].darkness = Number.isFinite(parsed)
            ? Math.max(0, Math.min(1, parsed))
            : null;
        figurine.images = [...figurine.images];
    }

    // The global keyhole darkness shows through when no per-image override is set,
    // so the stepper lands on it rather than a bare default.
    function darknessValue(value: number | null | undefined): number {
        if (typeof value === 'number' && Number.isFinite(value)) return Math.max(0, Math.min(1, value));
        const global = $themeConfig.effects?.keyholeDarkness;
        return typeof global === 'number' && Number.isFinite(global) ? global : 0.88;
    }

    // "Window" / "Shadow" are stepper buttons, not sliders: a range input cannot
    // shrink below its intrinsic width, so it overflowed and went dead in the
    // narrow per-image column. Each tap nudges by a fixed step, clamped to the
    // same bounds the renderer enforces.
    const REVEAL_MIN = 0.08, REVEAL_MAX = 1, DARK_MIN = 0.4, DARK_MAX = 1, KEYHOLE_STEP = 0.05;
    function nudgeRevealRadius(imgIdx: number, delta: number) {
        const next = revealRadiusValue(figurine.images[imgIdx].revealRadius) + delta;
        setRevealRadius(imgIdx, String(Math.round(next * 100) / 100));
    }
    function nudgeDarkness(imgIdx: number, delta: number) {
        const next = Math.max(DARK_MIN, Math.min(DARK_MAX, darknessValue(figurine.images[imgIdx].darkness) + delta));
        setDarkness(imgIdx, String(Math.round(next * 100) / 100));
    }

    function deriveImageVariants(url: string): { originalUrl: string | null; thumbUrl: string | null } {
        const marker = 'images/preview/';
        const idx = url.indexOf(marker);
        if (idx === -1) return { originalUrl: null, thumbUrl: null };
        const prefix = url.slice(0, idx);
        const fileName = url.slice(idx + marker.length);
        return {
            originalUrl: `${prefix}images/original/${fileName}`,
            thumbUrl: `${prefix}images/thumb/${fileName}`
        };
    }

    function moveImage(index: number, direction: -1 | 1) {
        const newIdx = index + direction;
        if (newIdx < 0 || newIdx >= figurine.images.length) return;
        const imgs = [...figurine.images];
        [imgs[index], imgs[newIdx]] = [imgs[newIdx], imgs[index]];
        figurine.images = imgs;
    }

    function addProcessStep() {
        figurine.processSteps = [...figurine.processSteps, {
            id: crypto.randomUUID(), stepType: 'sketch', description: '', imageUrl: ''
        }];
    }

    function removeProcessStep(index: number) {
        figurine.processSteps = figurine.processSteps.filter((_, i) => i !== index);
    }

    function setFaceImage(imageId: string) {
        figurine.images = figurine.images.map(img => ({
            ...img, imageType: img.id === imageId ? 'face' : 'full'
        }));
    }

    // "Second angle" — the home gallery card's hover reveal. Independent of
    // the cover/face image: exactly one image may carry it, any previous
    // holder demotes to 'full', and the cover image itself can't double as it
    // (the keyhole reveal already owns the face image).
    function setDetailImage(imageId: string) {
        figurine.images = figurine.images.map(img => {
            if (img.id === imageId) return { ...img, imageType: 'detail' };
            if (img.imageType === 'detail') return { ...img, imageType: 'full' };
            return img;
        });
    }

    function clearDetailImage(imageId: string) {
        figurine.images = figurine.images.map(img =>
            img.id === imageId ? { ...img, imageType: 'full' } : img
        );
    }

    function altTextLen(text: string | null | undefined): number {
        return (text ?? '').trim().length;
    }

    // Fills the [type]+[subject]+[material]+[context] SEO formula (same one the public
    // detail page falls back to automatically) into the alt text field, so an admin
    // gets a solid starting point instead of a blank input — see figurine-alt.ts.
    // Overwrites on purpose: it's an explicit click, not a silent fallback.
    function autoFillAlt(imgIdx: number) {
        const img = figurine.images[imgIdx];
        if (!img) return;
        figurine.images[imgIdx] = {
            ...img,
            altText: formatFigurineAlt(
                figurine,
                img.imageType,
                altLabelsFrom($t),
                siblingPosition(figurine.images, img),
            ),
        };
    }


  // The two side-channel texts live on their own endpoints, so they are fetched
  // when a work opens rather than arriving with the figurine payload.
  // Keyed on the id ALONE: `figurines` is read untracked so that a save (which
  // reloads the registry) doesn't re-run this and wipe an edit in progress.
  $effect(() => {
    const id = figurine.id;
    captionText = '';
    pinterestDescText = '';
    // Unsaved work (new or duplicated) — nothing stored under this id yet.
    if (!untrack(() => figurines).some((f) => f.id === id)) return;
    captionLoading = true;
    pinterestDescLoading = true;
    void (async () => {
      try {
        const c = await api.getFigurineCaption(id);
        // Guard: another work may have been opened while this was in flight.
        if (figurine.id === id) captionText = c ?? '';
      } catch { /* leave the field empty on failure */ }
      finally { if (figurine.id === id) captionLoading = false; }
      try {
        const pd = await api.getFigurinePinterestDescription(id);
        if (figurine.id === id) pinterestDescText = pd ?? '';
      } catch { /* leave the field empty on failure */ }
      finally { if (figurine.id === id) pinterestDescLoading = false; }
    })();
  });

  async function save() {
    isSaving = true;
    try {
      // Commit an open inline showing form first. Invalid data → flush() returns
      // false and shows its own inline error; abort so the author can fix it.
      if (showingsEditor && !(await showingsEditor.flush())) return;
      await api.saveFigurine(figurine);
      onMessage($t('adminMsgSavedServer'), 'success');
      await onSaved();
    } catch (e) {
      onMessage($t('adminMsgError') + e, 'error');
    } finally {
      isSaving = false;
    }
  }
</script>


<!-- ── TOP BAR ──────────────────────────────────────────── -->
<div class="shrink-0 px-5 py-3 border-b border-[#34251c]/10 bg-[#f2e8da] flex items-center gap-3 min-w-0">
    <!-- Delete (far left, subtle) -->
    <button
        onclick={() => onDelete()}
        disabled={deleting}
        class="shrink-0 text-[10px] uppercase tracking-wide text-[#5f4636]/40 hover:text-red-700 transition-colors"
        title={$t('adminFormDeleteWork')}>✕</button>

    <!-- Name -->
    <input
        bind:value={figurine.name}
        class="flex-1 min-w-0 bg-transparent border-0 border-b border-[#34251c]/20 focus:border-[#c65f3c]/50 outline-none text-sm font-bold text-[#34251c] px-1 py-0.5 transition-colors"
        placeholder={$t('adminFieldName')} />

    <!-- Status -->
    <select bind:value={figurine.status}
        class="shrink-0 text-[10px] uppercase tracking-wide bg-[#f8f1e7] border border-[#34251c]/15 px-2 py-1.5 text-[#34251c] outline-none focus:border-[#c65f3c]/40 transition-colors">
        <option value="available">{$t('adminFieldStatusAvail')}</option>
        <option value="reserved">{$t('adminFieldStatusRes')}</option>
        <option value="in_progress">{$t('adminFieldStatusWip')}</option>
        <option value="sold">{$t('adminFieldStatusSold')}</option>
    </select>

    <!-- Visible -->
    <label class="flex items-center gap-1.5 shrink-0 cursor-pointer select-none">
        <input type="checkbox" bind:checked={figurine.isVisible} class="accent-[#34251c] w-3.5 h-3.5" />
        <span class="text-[10px] uppercase tracking-wide text-[#5f4636]">{$t('adminFieldVisible')}</span>
    </label>

    <!-- Featured -->
    <label class="flex items-center gap-1.5 shrink-0 cursor-pointer select-none">
        <input type="checkbox" bind:checked={figurine.isFeatured} class="accent-[#c65f3c] w-3.5 h-3.5" />
        <span class="text-[10px] uppercase tracking-wide text-[#5f4636]">{$t('adminFieldFeatured')}</span>
    </label>

    <!-- Unsaved pulse dot -->
    {#if unsaved}
        <span class="w-1.5 h-1.5 rounded-full bg-amber-600 animate-pulse shrink-0" title={$t('adminRegistryUnsaved')}></span>
    {/if}

    <!-- Cancel -->
    <button onclick={onCancel} class="btn-gothic text-[10px] shrink-0 opacity-70">{$t('adminFormCancel')}</button>

    <!-- Save -->
    <button onclick={save} disabled={isSaving}
        class="btn-gothic text-[10px] shrink-0 min-w-[90px] transition-colors
            {unsaved ? 'bg-amber-50 border-amber-700/40 text-amber-900 hover:bg-amber-100' : 'bg-[#34251c]/10'}">
        {isSaving ? $t('adminFormSaving') : unsaved ? $t('adminFormSaveChanges') : $t('adminFormSaved')}
    </button>
</div>

<!-- ── TAB STRIP ─────────────────────────────────────────── -->
<div class="shrink-0 flex border-b border-[#34251c]/10 bg-[#f8f1e7] px-1">
    {#each [
        ['media',    $t('adminFormTabMedia')],
        ['text',     $t('adminFormTabText')],
        ['object',   $t('adminFormTabObject')],
        ['passport', $t('adminFormTabPassport')],
        ['vitrina',  $t('adminFormTabVitrina')],
    ] as tab}
        <button
            onclick={() => activeFormTab = tab[0] as typeof activeFormTab}
            class="px-5 py-2.5 text-[10px] uppercase tracking-wide border-b-2 -mb-px transition-colors
                {activeFormTab === tab[0]
                    ? 'border-[#c65f3c] text-[#34251c]'
                    : 'border-transparent text-[#5f4636] hover:text-[#34251c]'}"
        >{tab[1]}</button>
    {/each}
</div>

<!-- ── TAB CONTENT (scrollable) ──────────────────────────── -->
<div class="flex-1 overflow-y-auto">

    <!-- ╔═ MEDIA ════════════════════════════════════════════ -->
    {#if activeFormTab === 'media'}
    <div class="p-6 space-y-8" in:fade={{ duration: 120 }}>

        <!-- Gallery header -->
        <div>
            <div class="flex items-center justify-between mb-4">
                <span class="label">{$t('adminMediaPhotos')} ({figurine.images.length})</span>
                <div class="flex gap-2 flex-wrap justify-end">
                    {#if figurine.images.length > 0}
                        <button onclick={generateDepth} disabled={generatingDepth}
                            title={$t('adminMediaDepthHint')}
                            class="btn-gothic text-[10px] disabled:opacity-60 disabled:cursor-wait">
                            {generatingDepth ? $t('adminMediaDepthGenRunning') : $t('adminMediaDepthGen')}
                        </button>
                    {/if}
                    <button onclick={() => handlePickFile('images')} class="btn-gothic text-[10px]" disabled={!!folderUploadProgress}>{$t('adminMediaAddPhoto')}</button>
                    <button onclick={handleFolderUpload} class="btn-gothic text-[10px]" disabled={!!folderUploadProgress}>
                        {folderUploadProgress
                            ? $t('adminMediaFolderProgress').replace('{done}', String(folderUploadProgress.done)).replace('{total}', String(folderUploadProgress.total))
                            : $t('adminMediaAddFolder')}
                    </button>
                </div>
            </div>

            <!-- Thumbnails — 144px, click to expand -->
            <div class="flex flex-wrap gap-3 mb-4">
                {#each figurine.images as img, imgIdx (img.id)}
                    <div
                        role="button"
                        tabindex="0"
                        onclick={() => selectedImageIdx = selectedImageIdx === imgIdx ? null : imgIdx}
                        onkeydown={(e) => e.key === 'Enter' && (selectedImageIdx = selectedImageIdx === imgIdx ? null : imgIdx)}
                        class="relative w-36 h-36 border-2 overflow-hidden transition-all group/thumb shrink-0 cursor-pointer
                            {selectedImageIdx === imgIdx
                                ? 'border-[#c65f3c] shadow-[0_0_0_2px_rgba(198,95,60,0.2)]'
                                : img.imageType === 'face'
                                    ? 'border-amber-500 hover:border-amber-600'
                                    : 'border-[#34251c]/20 hover:border-[#34251c]/50'}">
                        <img src={resolveUrl(img.thumbUrl ?? img.url)} alt={img.altText ?? ''} class="w-full h-full object-cover pointer-events-none" />

                        <!-- Move arrows on hover -->
                        <div class="absolute bottom-1 inset-x-0 flex justify-center gap-1 opacity-0 group-hover/thumb:opacity-100 transition-opacity">
                            <button
                                onclick={(e) => { e.stopPropagation(); moveImage(imgIdx, -1); }}
                                disabled={imgIdx === 0}
                                class="bg-[#fff9f0]/90 text-[#34251c] text-[10px] px-2 py-0.5 border border-[#34251c]/20 disabled:opacity-30 hover:bg-[#f8f1e7]">←</button>
                            <button
                                onclick={(e) => { e.stopPropagation(); moveImage(imgIdx, 1); }}
                                disabled={imgIdx === figurine.images.length - 1}
                                class="bg-[#fff9f0]/90 text-[#34251c] text-[10px] px-2 py-0.5 border border-[#34251c]/20 disabled:opacity-30 hover:bg-[#f8f1e7]">→</button>
                        </div>

                        {#if img.imageType === 'face'}
                            <div class="absolute bottom-0 left-0 right-0 bg-amber-500/80 text-black text-[8px] text-center py-0.5 font-bold pointer-events-none">{$t('adminMediaCoverBadge')}</div>
                        {:else if img.imageType === 'detail'}
                            <div class="absolute bottom-0 left-0 right-0 bg-[#c65f3c]/85 text-[#fff7ea] text-[8px] text-center py-0.5 font-bold pointer-events-none">{$t('adminMediaDetailBadge')}</div>
                        {/if}
                        {#if img.depthUrl}
                            <div class="absolute top-0 left-0 bg-[#34251c]/85 text-[#f3e9d8] text-[8px] px-1 py-0.5 leading-none tracking-wider font-bold pointer-events-none">{$t('adminMediaDepthBadge')}</div>
                        {/if}
                    </div>
                {/each}

                {#if figurine.images.length === 0}
                    <p class="text-xs text-[#5f4636]/50 italic py-6 w-full text-center">{$t('adminGrimoireEmpty')}</p>
                {/if}
            </div>

            <!-- Per-image expanded panel (full width) -->
            {#if selectedImageIdx !== null && figurine.images[selectedImageIdx]}
                {@const img = figurine.images[selectedImageIdx]}
                {@const imgIdx = selectedImageIdx}
                <div class="border border-[#c65f3c]/25 bg-[#f8f1e7] p-5" in:slide={{ duration: 160 }}>

                    <!-- Panel header -->
                    <div class="flex items-center justify-between mb-5">
                        <span class="text-[10px] uppercase tracking-wide text-[#5f4636]">
                            {$t('adminMediaPhotoN').replace('{n}', String(imgIdx + 1)).replace('{total}', String(figurine.images.length))}
                        </span>
                        <div class="flex items-center gap-4">
                            {#if img.imageType !== 'face'}
                                <button onclick={() => setFaceImage(img.id)}
                                    class="text-[10px] uppercase tracking-wide text-[#5f4636] hover:text-amber-800 transition-colors">{$t('adminMediaCover')}</button>
                            {:else}
                                <span class="text-[10px] uppercase tracking-wide text-amber-700">{$t('adminMediaCover')} ✓</span>
                            {/if}
                            {#if img.imageType === 'detail'}
                                <button onclick={() => clearDetailImage(img.id)}
                                    class="text-[10px] uppercase tracking-wide text-[#c65f3c]">{$t('adminMediaDetail')} ✓ · {$t('adminMediaDetailClear')}</button>
                            {:else if img.imageType !== 'face'}
                                <button onclick={() => setDetailImage(img.id)} title={$t('adminMediaDetailHint')}
                                    class="text-[10px] uppercase tracking-wide text-[#5f4636] hover:text-[#c65f3c] transition-colors">{$t('adminMediaDetail')}</button>
                            {/if}
                            <button onclick={() => {
                                    figurine.images = figurine.images.filter(i => i.id !== img.id);
                                    selectedImageIdx = null;
                                }}
                                class="text-[10px] uppercase tracking-wide text-red-700 hover:text-red-900 transition-colors">{$t('adminMediaDeleteFile')}</button>
                            <button onclick={() => selectedImageIdx = null}
                                class="text-[10px] text-[#5f4636] hover:text-[#34251c] transition-colors">✕</button>
                        </div>
                    </div>

                    <div class="grid grid-cols-2 gap-8">
                        <!-- Left: preview + alt + depth -->
                        <div class="space-y-4">
                            <div class="border border-[#34251c]/15 overflow-hidden bg-[#f1e3d1] aspect-square">
                                <img src={resolveUrl(img.url)} alt={img.altText ?? ''} class="w-full h-full object-contain" />
                            </div>
                            <label class="block">
                                <div class="flex items-center justify-between gap-2">
                                    <span class="label">{$t('adminMediaAltPlaceholder')}</span>
                                    <button type="button" onclick={() => autoFillAlt(imgIdx)}
                                        class="text-[9px] uppercase tracking-wide text-[#5f4636] hover:text-[#c65f3c] transition-colors shrink-0">
                                        {$t('adminMediaAltAuto')}
                                    </button>
                                </div>
                                <input bind:value={img.altText} type="text" class="input-gothic text-xs" />
                                <div class="flex items-center justify-between mt-1">
                                    <span class="text-[9px] text-[#5f4636]/70 leading-snug">{$t('adminMediaAltFormulaHint')}</span>
                                    <span class="text-[9px] shrink-0 ml-2 {altTextLen(img.altText) === 0 ? 'text-[#5f4636]/50' : altTextLen(img.altText) < 50 || altTextLen(img.altText) > 125 ? 'text-[#c65f3c]' : 'text-emerald-700'}">
                                        {altTextLen(img.altText)}{altTextLen(img.altText) > 0 ? (altTextLen(img.altText) < 50 ? ` (${$t('adminMediaAltTooShort')})` : altTextLen(img.altText) > 125 ? ` (${$t('adminMediaAltTooLong')})` : '') : ''}
                                    </span>
                                </div>
                            </label>
                            <!-- Depth map -->
                            <div>
                                <span class="label">{$t('adminMediaDepthAdd')}</span>
                                {#if img.depthUrl}
                                    <div class="flex items-center gap-2 mt-1">
                                        <img src={resolveUrl(img.depthUrl)} alt="" class="w-10 h-10 object-cover border border-[#34251c]/20 shrink-0" />
                                        <button onclick={() => handlePickDepth(imgIdx)} class="btn-gothic text-[10px]">{$t('adminMediaDepthReplace')}</button>
                                        <button onclick={() => clearDepth(imgIdx)} class="text-[10px] uppercase text-red-700 hover:text-red-900">✕</button>
                                    </div>
                                {:else}
                                    <button onclick={() => handlePickDepth(imgIdx)}
                                        class="mt-1 w-full btn-gothic text-[10px] border-dashed">{$t('adminMediaDepthAdd')}</button>
                                {/if}
                            </div>
                        </div>

                        <!-- Right: parallax + keyhole -->
                        <div class="space-y-6">
                            <!-- Parallax -->
                            <div>
                                <div class="flex items-center justify-between mb-2">
                                    <span class="label">{$t('adminMediaParallax')}</span>
                                    <button onclick={() => resetParallaxIntensity(imgIdx)}
                                        disabled={img.parallaxIntensity == null}
                                        class="text-[9px] uppercase text-[#5f4636] hover:text-[#34251c] disabled:opacity-30 transition-colors">{$t('adminMediaParallaxReset')}</button>
                                </div>
                                <div class="flex items-center gap-3">
                                    <input type="range" min="0" max="1" step="0.05"
                                        value={parallaxValue(img.parallaxIntensity)}
                                        oninput={(e) => setParallaxIntensity(imgIdx, (e.currentTarget as HTMLInputElement).value)}
                                        class="flex-1 accent-[#6f3b24]" />
                                    <span class="w-10 text-right text-xs tabular-nums text-[#5f4636]">{parallaxValue(img.parallaxIntensity).toFixed(2)}</span>
                                </div>
                            </div>

                            <!-- Keyhole (cover image only) -->
                            {#if img.imageType === 'face'}
                                <div>
                                    <div class="flex items-center justify-between mb-3">
                                        <span class="label">{$t('adminMediaKeyhole')}</span>
                                        <button onclick={() => resetReveal(imgIdx)}
                                            disabled={img.focalX == null && img.focalY == null && img.revealRadius == null && img.darkness == null}
                                            class="text-[9px] uppercase text-[#5f4636] hover:text-[#34251c] disabled:opacity-30 transition-colors">{$t('adminMediaParallaxReset')}</button>
                                    </div>
                                    <!-- Focal point picker — 280×210, much more usable than 112×84 -->
                                    <div class="relative border border-[#34251c]/20 overflow-hidden bg-[#f1e3d1] mb-4" style="width: 280px; aspect-ratio: 4/3;">
                                        <img src={resolveUrl(img.thumbUrl ?? img.url)} alt="" class="w-full h-full object-contain" />
                                        <KeyholeVeil
                                            focalX={img.focalX}
                                            focalY={img.focalY}
                                            revealRadius={img.revealRadius}
                                            darkness={darknessValue(img.darkness)}
                                            editable
                                            onpick={(x, y) => setFocalPoint(imgIdx, x, y)}
                                        />
                                    </div>
                                    <!-- Window size stepper -->
                                    <div class="flex items-center gap-3 mb-3">
                                        <span class="text-[10px] uppercase tracking-wide text-[#5f4636] w-16 shrink-0">{$t('adminMediaKeyholeRadius')}</span>
                                        <div class="flex items-center flex-1 border border-[#34251c]/20 bg-[#fff9f0]">
                                            <button type="button" onclick={() => nudgeRevealRadius(imgIdx, -KEYHOLE_STEP)}
                                                disabled={revealRadiusValue(img.revealRadius) <= REVEAL_MIN}
                                                class="px-3 py-1.5 text-sm text-[#5f4636] hover:bg-[#34251c]/10 disabled:opacity-30">−</button>
                                            <span class="flex-1 text-center text-xs tabular-nums text-[#5f4636]">{revealRadiusValue(img.revealRadius).toFixed(2)}</span>
                                            <button type="button" onclick={() => nudgeRevealRadius(imgIdx, KEYHOLE_STEP)}
                                                disabled={revealRadiusValue(img.revealRadius) >= REVEAL_MAX}
                                                class="px-3 py-1.5 text-sm text-[#5f4636] hover:bg-[#34251c]/10 disabled:opacity-30">+</button>
                                        </div>
                                    </div>
                                    <!-- Darkness stepper -->
                                    <div class="flex items-center gap-3">
                                        <span class="text-[10px] uppercase tracking-wide text-[#5f4636] w-16 shrink-0">{$t('adminMediaDarkness')}</span>
                                        <div class="flex items-center flex-1 border border-[#34251c]/20 bg-[#fff9f0]">
                                            <button type="button" onclick={() => nudgeDarkness(imgIdx, -KEYHOLE_STEP)}
                                                disabled={darknessValue(img.darkness) <= DARK_MIN}
                                                class="px-3 py-1.5 text-sm text-[#5f4636] hover:bg-[#34251c]/10 disabled:opacity-30">−</button>
                                            <span class="flex-1 text-center text-xs tabular-nums {img.darkness == null ? 'text-[#5f4636]/45 italic' : 'text-[#5f4636]'}">{darknessValue(img.darkness).toFixed(2)}</span>
                                            <button type="button" onclick={() => nudgeDarkness(imgIdx, KEYHOLE_STEP)}
                                                disabled={darknessValue(img.darkness) >= DARK_MAX}
                                                class="px-3 py-1.5 text-sm text-[#5f4636] hover:bg-[#34251c]/10 disabled:opacity-30">+</button>
                                        </div>
                                    </div>
                                </div>
                            {/if}
                        </div>
                    </div>
                </div>
            {/if}
        </div>

        <!-- Video + Audio -->
        <div class="grid grid-cols-2 gap-6">
            <!-- Video -->
            <div class="p-4 border border-dashed border-[#34251c]/20 flex flex-col gap-2">
                <span class="label block">{$t('adminMediaVideo')}</span>
                {#if figurine.videoUrl}
                    <video src={resolveUrl(figurine.videoUrl)} controls class="w-full max-h-36 bg-[#2f2117]" preload="metadata">
                        <track kind="captions" />
                    </video>
                    <div class="flex gap-2">
                        <button onclick={() => handlePickFile('videos')} disabled={uploadingVideo}
                            class="text-[10px] text-[#34251c]/85 hover:text-[#6f3b24] uppercase disabled:opacity-70">{$t('adminMediaReplace')}</button>
                        <button onclick={() => { figurine.videoUrl = null; externalVideoUrl = ''; }}
                            class="text-[10px] text-red-700 hover:text-red-900 uppercase">{$t('adminMediaDeleteFile')}</button>
                    </div>
                {:else}
                    <div class="flex flex-col gap-2">
                        <input type="url" bind:value={externalVideoUrl} placeholder="https://... external link" class="input-gothic text-xs" />
                        {#if externalVideoUrl.trim()}
                            <button onclick={() => { figurine.videoUrl = externalVideoUrl.trim(); externalVideoUrl = ''; }}
                                class="btn-gothic text-xs w-full">{$t('adminMediaUseLink')}</button>
                        {:else}
                            <button onclick={() => handlePickFile('videos')} disabled={uploadingVideo}
                                class="btn-gothic text-xs w-full disabled:opacity-70">{uploadingVideo ? '…' : $t('adminMediaPickMp4')}</button>
                        {/if}
                    </div>
                {/if}
            </div>
            <!-- Audio -->
            <div class="p-4 border border-dashed border-[#34251c]/20 flex flex-col gap-2">
                <span class="label block">{$t('adminMediaAudio')}</span>
                {#if figurine.ambiencePath}
                    <audio src={resolveUrl(figurine.ambiencePath)} controls class="w-full" preload="metadata"></audio>
                    <div class="flex gap-2">
                        <button onclick={() => handlePickFile('audio')} disabled={uploadingAudio}
                            class="text-[10px] text-[#34251c]/85 hover:text-[#6f3b24] uppercase disabled:opacity-70">{$t('adminMediaReplace')}</button>
                        <button onclick={() => figurine.ambiencePath = null}
                            class="text-[10px] text-red-700 hover:text-red-900 uppercase">{$t('adminMediaDeleteFile')}</button>
                    </div>
                {:else}
                    <button onclick={() => handlePickFile('audio')} disabled={uploadingAudio}
                        class="btn-gothic text-xs w-full disabled:opacity-70">{uploadingAudio ? '…' : $t('adminMediaPickMp3')}</button>
                {/if}
            </div>
        </div>

    </div>

    <!-- ╔═ TEXT ═════════════════════════════════════════════ -->
    {:else if activeFormTab === 'text'}
    <div class="p-8 max-w-3xl space-y-6" in:fade={{ duration: 120 }}>

        <label class="block">
            <span class="label">{$t('adminFieldQuote')}</span>
            <textarea bind:value={figurine.shortText} class="input-gothic h-24"></textarea>
        </label>

        <label class="block">
            <span class="label">{$t('adminFieldHistory')}</span>
            <textarea bind:value={figurine.fullDescription} class="input-gothic h-56"></textarea>
        </label>

        <label class="block opacity-70">
            <span class="label">{$t('adminFieldSecret')}</span>
            <textarea bind:value={figurine.secretText} class="input-gothic h-20"></textarea>
        </label>

        <div class="block border-t border-[#34251c]/10 pt-6">
            <span class="label">{$t('adminGazetteAnnounceHeading')}</span>
            <p class="text-[11px] leading-snug text-[#7c6554] mb-2">{$t('adminGazetteAnnounceHint')}</p>
            {#if figurineExists}
                <label class="block mb-3">
                    <span class="label">{$t('adminGazetteTitleEn')}</span>
                    <input bind:value={gzTitle} class="input-gothic" />
                </label>
                <label class="block mb-3">
                    <span class="label">{$t('adminGazetteDekEn')}</span>
                    <textarea bind:value={gzSummary} class="input-gothic h-28"></textarea>
                </label>
                <button type="button" onclick={publishGazetteNote} disabled={gzBusy}
                    class="px-4 py-2 border border-[#34251c]/25 hover:border-[#34251c]/55 text-[#5f4636] hover:text-[#34251c] text-xs tracking-wide uppercase transition-colors disabled:opacity-40">
                    {gzBusy ? $t('adminFormSaving') : $t('adminGazetteAnnouncePublish')}
                </button>
            {:else}
                <p class="text-[11px] italic text-[#7c6554]">{$t('adminCaptionSaveNewFirst')}</p>
            {/if}
        </div>

        <div class="block border-t border-[#34251c]/10 pt-6">
            <span class="label">{$t('adminCaptionLabel')}</span>
            <p class="text-[11px] leading-snug text-[#7c6554] mb-2">{$t('adminCaptionHint')}</p>
            {#if figurineExists}
                <textarea bind:value={captionText} disabled={captionLoading}
                    placeholder={captionLoading ? '…' : ''}
                    class="input-gothic h-28"></textarea>
                <div class="mt-2">
                    <button type="button" onclick={saveCaption} disabled={captionSaving || captionLoading}
                        class="px-4 py-2 border border-[#34251c]/25 hover:border-[#34251c]/55 text-[#5f4636] hover:text-[#34251c] text-xs tracking-wide uppercase transition-colors disabled:opacity-40">
                        {captionSaving ? '…' : $t('adminCaptionSave')}
                    </button>
                </div>
            {:else}
                <p class="text-[11px] italic text-[#7c6554]">{$t('adminCaptionSaveNewFirst')}</p>
            {/if}
        </div>

        <div class="block border-t border-[#34251c]/10 pt-6">
            <span class="label">{$t('adminPinterestDescLabel')}</span>
            <p class="text-[11px] leading-snug text-[#7c6554] mb-2">{$t('adminPinterestDescHint')}</p>
            {#if figurineExists}
                <textarea bind:value={pinterestDescText} disabled={pinterestDescLoading}
                    placeholder={pinterestDescLoading ? '…' : ''}
                    class="input-gothic h-28"></textarea>
                <div class="mt-2 flex gap-2">
                    <button type="button" onclick={generatePinterestDesc} disabled={pinterestDescLoading}
                        class="px-4 py-2 border border-[#34251c]/25 hover:border-[#34251c]/55 text-[#5f4636] hover:text-[#34251c] text-xs tracking-wide uppercase transition-colors disabled:opacity-40">
                        {$t('adminPinterestDescGenerate')}
                    </button>
                    <button type="button" onclick={savePinterestDesc} disabled={pinterestDescSaving || pinterestDescLoading}
                        class="px-4 py-2 border border-[#34251c]/25 hover:border-[#34251c]/55 text-[#5f4636] hover:text-[#34251c] text-xs tracking-wide uppercase transition-colors disabled:opacity-40">
                        {pinterestDescSaving ? '…' : $t('adminPinterestDescSave')}
                    </button>
                </div>
            {:else}
                <p class="text-[11px] italic text-[#7c6554]">{$t('adminPinterestDescSaveNewFirst')}</p>
            {/if}
        </div>

    </div>

    <!-- ╔═ OBJECT ═══════════════════════════════════════════ -->
    {:else if activeFormTab === 'object'}
    <div class="p-8 max-w-2xl space-y-5" in:fade={{ duration: 120 }}>

        <div class="grid grid-cols-3 gap-4">
            <label class="block">
                <span class="label">{$t('adminFieldYear')}</span>
                <input type="number" bind:value={figurine.year} class="input-gothic" />
            </label>
            <label class="block">
                <span class="label">{$t('adminFieldSeries')}</span>
                <input bind:value={figurine.series} class="input-gothic" placeholder="—" />
            </label>
            <label class="block">
                <span class="label">{$t('adminFieldSortOrder')}</span>
                <input type="number" bind:value={figurine.sortOrder} class="input-gothic" />
            </label>
        </div>

        <label class="block">
            <span class="label">{$t('adminFieldSlug')}</span>
            <input bind:value={figurine.slug} class="input-gothic" placeholder={$t('adminFieldSlugPlaceholder')} autocomplete="off" spellcheck="false" />
            <span class="block mt-1 text-[10px] text-[#5f4636]/60">{$t('adminFieldSlugHint')}</span>
        </label>

        <label class="block">
            <span class="label">{$t('adminFieldMaterial')}</span>
            <input bind:value={figurine.material} class="input-gothic" list="suggest-material" autocomplete="off" />
            <datalist id="suggest-material">
                {#each materialSuggestions as s}<option value={s}></option>{/each}
            </datalist>
        </label>

        <label class="block">
            <span class="label">{$t('adminFieldTechnique')}</span>
            <input bind:value={figurine.technique} class="input-gothic" list="suggest-technique" autocomplete="off" />
            <datalist id="suggest-technique">
                {#each techniqueSuggestions as s}<option value={s}></option>{/each}
            </datalist>
        </label>

        <label class="block">
            <span class="label">{$t('adminFieldDimensions')}</span>
            <input bind:value={figurine.dimensions} class="input-gothic" placeholder="20×15×10 cm" list="suggest-dimensions" autocomplete="off" />
            <datalist id="suggest-dimensions">
                {#each dimensionsSuggestions as s}<option value={s}></option>{/each}
            </datalist>
        </label>

    </div>

    <!-- ╔═ PASSPORT ══════════════════════════════════════════ -->
    {:else if activeFormTab === 'passport'}
    <div class="p-8 max-w-3xl" in:fade={{ duration: 120 }}>
        <p class="text-xs text-[#5f4636]/70 mb-6 max-w-prose leading-relaxed">{$t('adminPassportHint')}</p>

        <div class="grid grid-cols-3 gap-4 mb-6">
            <label class="block">
                <span class="label">{$t('passportNumber')}</span>
                <input bind:value={figurine.passportNumber} class="input-gothic" placeholder="RTN-2026-001" />
            </label>
            <label class="block">
                <span class="label">{$t('passportEdition')}</span>
                <input bind:value={figurine.edition} class="input-gothic" placeholder="1 of 1" />
            </label>
            <label class="block">
                <span class="label">{$t('passportCreated')}</span>
                <input bind:value={figurine.createdPeriod} class="input-gothic" placeholder="Spring 2026" />
            </label>
        </div>

        <div class="grid grid-cols-2 gap-5">
            <label class="block">
                <span class="label">{$t('passportProvenance')}</span>
                <textarea bind:value={figurine.provenanceNote} class="input-gothic h-28"></textarea>
            </label>
            <label class="block">
                <span class="label">{$t('passportAuthenticity')}</span>
                <textarea bind:value={figurine.authenticityNote} class="input-gothic h-28"></textarea>
            </label>
            <label class="block">
                <span class="label">{$t('passportCare')}</span>
                <textarea bind:value={figurine.careInstructions} class="input-gothic h-28"></textarea>
            </label>
            <label class="block">
                <span class="label">{$t('passportIncluded')}</span>
                <textarea bind:value={figurine.includedItems} class="input-gothic h-28"></textarea>
            </label>
        </div>
    </div>

    <!-- ╔═ VITRINA ═══════════════════════════════════════════ -->
    {:else if activeFormTab === 'vitrina'}
    <div class="p-8 max-w-3xl space-y-8" in:fade={{ duration: 120 }}>

        <!-- Display layout -->
        <label class="block max-w-xs">
            <span class="label">{$t('adminFieldLayout')}</span>
            <select bind:value={figurine.displayLayout} class="input-gothic">
                <option value={null}>{$t('adminFieldLayoutSpecimen')}</option>
                <option value="showcase">{$t('adminFieldLayoutShowcase')}</option>
                <option value="codex">{$t('adminFieldLayoutCodex')}</option>
                <option value="diptych">{$t('adminFieldLayoutDiptych')}</option>
                <option value="broadside">{$t('adminFieldLayoutBroadside')}</option>
            </select>
        </label>

        <!-- Display config -->
        <div>
            <span class="label">{$t('adminDisplayConfig')}</span>
            <DisplayConfigEditor bind:value={figurine.displayConfig as (string | null)} />
        </div>

        <!-- Showing window -->
        <div class="border-t border-[#34251c]/10 pt-6">
            <span class="label block mb-3">{$t('adminFieldShowingWindow')}</span>
            <select
                value={figWindowMode(figurine)}
                onchange={(e) => setFigWindowMode(e.currentTarget.value)}
                class="input-gothic max-w-sm mb-3">
                <option value="">{$t('adminShowingModeAlways')}</option>
                <option value="custom">{$t('adminShowingModeCustom')}</option>
                {#each showingRooms as room (room.id)}
                    {#if room.name}
                        <option value={room.id}>{room.name} ({minutesToClock(room.openFromMin)}–{minutesToClock(room.openUntilMin)})</option>
                    {/if}
                {/each}
            </select>

            {#if figWindowMode(figurine) === 'custom'}
                <div class="flex gap-4 mb-3 max-w-sm">
                    <label class="block flex-1">
                        <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldShowingFrom')}</span>
                        <input type="time" value={minutesToClock(figurine.openFromMin)}
                            oninput={(e) => figurine.openFromMin = clockToMinutes(e.currentTarget.value)}
                            class="input-gothic" />
                    </label>
                    <label class="block flex-1">
                        <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFieldShowingUntil')}</span>
                        <input type="time" value={minutesToClock(figurine.openUntilMin)}
                            oninput={(e) => figurine.openUntilMin = clockToMinutes(e.currentTarget.value)}
                            class="input-gothic" />
                    </label>
                </div>
                <p class="text-[10px] text-[#7c6554] mb-3 leading-snug">{$t('adminFieldShowingHint')}</p>
            {/if}

            <!-- First look: timed early-release for book-holders -->
            <label class="block max-w-sm mb-1">
                <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminFirstLookLabel')}</span>
                <div class="flex items-center gap-2">
                    <input type="datetime-local"
                        value={figurine.firstLookUntil ? toLocalInput(new Date(figurine.firstLookUntil)) : ''}
                        oninput={(e) => figurine.firstLookUntil = e.currentTarget.value ? new Date(e.currentTarget.value).toISOString() : null}
                        class="input-gothic" />
                    {#if figurine.firstLookUntil}
                        <button type="button" class="text-[11px] uppercase tracking-wide text-[#6f3b24] whitespace-nowrap" onclick={() => figurine.firstLookUntil = null}>{$t('adminFirstLookClear')}</button>
                    {/if}
                </div>
            </label>
            <p class="text-[10px] text-[#7c6554] mb-4 leading-snug">{$t('adminFirstLookHint')}</p>

            <!-- Preview clock -->
            <div class="border-t border-[#34251c]/10 pt-4">
                <div class="flex flex-wrap items-end gap-3">
                    <label class="block">
                        <span class="text-[10px] uppercase tracking-wide text-[#7c6554]">{$t('adminPreviewAt')}</span>
                        <input type="datetime-local" value={toLocalInput(previewAt)}
                            oninput={(e) => { if (e.currentTarget.value) previewAt = new Date(e.currentTarget.value); }}
                            class="input-gothic" />
                    </label>
                    <button type="button" class="text-[11px] uppercase tracking-wide text-[#6f3b24] pb-2" onclick={() => previewAt = new Date()}>{$t('adminPreviewNow')}</button>
                    <span class="text-[10px] uppercase tracking-wide px-2 py-1 rounded pb-1 {previewFigOpen ? 'bg-emerald-600/15 text-emerald-700' : 'bg-[#6f3b24]/12 text-[#6f3b24]'}">
                        {previewFigOpen ? $t('adminPreviewOpen') : $t('adminPreviewClosed')}
                    </span>
                </div>
                {#if !previewFigOpen}
                    <div class="relative w-40 aspect-[3/4] mt-3 rounded-[3px] overflow-hidden border border-[#34251c]/15">
                        <SealedDoor
                            openFromMin={previewFigWindow.openFromMin}
                            openUntilMin={previewFigWindow.openUntilMin}
                            daysMask={previewFigWindow.daysMask}
                            monthDay={previewFigWindow.monthDay}
                            dateFrom={previewFigWindow.dateFrom}
                            dateUntil={previewFigWindow.dateUntil}
                            imageUrl={(figurine.images?.find(i => i.imageType === 'face') ?? figurine.images?.[0])?.url}
                            thumbUrl={(figurine.images?.find(i => i.imageType === 'face') ?? figurine.images?.[0])?.thumbUrl}
                            name={figurine.name}
                            now={previewAt}
                            compact
                        />
                    </div>
                {/if}
            </div>
        </div>

        <!-- Grimoire (process steps) -->
        <div class="border-t border-[#34251c]/10 pt-6">
            <div class="flex items-center justify-between mb-4">
                <h3 class="text-sm font-gothic">{$t('adminGrimoireHeading')}</h3>
                <button onclick={addProcessStep} class="btn-gothic text-[10px]">{$t('adminGrimoireAddStep')}</button>
            </div>
            <div class="space-y-3">
                {#each figurine.processSteps as step, i}
                    <div class="p-4 bg-[#f8f1e7] border border-[#34251c]/10 flex gap-4 items-start">
                        <div class="w-20 h-20 bg-[#f1e3d1] flex items-center justify-center border border-[#34251c]/20 relative group shrink-0">
                            {#if step.imageUrl}
                                <img src={resolveUrl(step.imageUrl)} alt="" class="w-full h-full object-cover" />
                                <button onclick={() => step.imageUrl = ''} class="absolute top-0 right-0 bg-[#6f3b24]/30 text-[#fff9f0] p-0.5 text-[9px] opacity-0 group-hover:opacity-100">✕</button>
                            {:else}
                                <button onclick={() => handlePickFile('images', i)} class="text-[10px] uppercase text-[#5f4636] hover:text-[#34251c]">{$t('adminGrimoirePhoto')}</button>
                            {/if}
                        </div>
                        <div class="flex-1 grid gap-2">
                            <select bind:value={step.stepType} class="input-gothic text-xs py-1.5">
                                <option value="sketch">Sketch</option>
                                <option value="prototype">Prototype</option>
                                <option value="modeling">Modeling</option>
                                <option value="painting">Painting</option>
                                <option value="finish">Finish</option>
                            </select>
                            <textarea bind:value={step.description} class="input-gothic h-14 text-xs" placeholder={$t('adminGrimoireStepDesc')}></textarea>
                        </div>
                        <button onclick={() => removeProcessStep(i)} class="text-[#5f4636] hover:text-red-500 self-center text-sm">✕</button>
                    </div>
                {/each}
                {#if figurine.processSteps.length === 0}
                    <div class="text-center text-[#5f4636] text-xs py-4 opacity-70">{$t('adminGrimoireEmpty')}</div>
                {/if}
            </div>
        </div>

        <!-- Showings for this figurine -->
        {#if figurine.id}
            <div class="border-t border-[#34251c]/10 pt-6">
                <FigurineShowingsEditor bind:this={showingsEditor} figurineId={figurine.id} />
            </div>
        {/if}

    </div>
    {/if}

</div>

<style>
    .label {
        font-size: 10px;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: #5f4636;
        margin-bottom: 0.35rem;
        display: block;
        font-weight: 700;
    }

    .input-gothic {
        width: 100%;
        background-color: #f8f1e7;
        border: 1px solid rgba(198, 95, 60, 0.2);
        padding: 0.65rem 0.75rem;
        font-size: 0.875rem;
        color: #34251c;
        outline: none;
        transition: border-color 0.2s;
        font-family: inherit;
    }

    .input-gothic:focus {
        border-color: rgba(198, 95, 60, 0.55);
    }

    textarea.input-gothic { resize: none; }

    .btn-gothic {
        padding: 0.45rem 1.25rem;
        border: 1px solid rgba(198, 95, 60, 0.3);
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        cursor: pointer;
        transition: all 0.2s;
        background: transparent;
        color: #34251c;
        font-family: inherit;
    }

    .btn-gothic:hover { background-color: rgba(198, 95, 60, 0.06); }
    .btn-gothic:disabled { opacity: 0.3; cursor: not-allowed; }
</style>
