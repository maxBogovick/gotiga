// See https://svelte.dev/docs/kit/types#app.d.ts
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}

		interface FigurineDetailContext {
			readonly figurine: import('$lib/types/api').Figurine;
			readonly id: string;
			readonly prev: import('$lib/types/api').FigurineListItem | null;
			readonly next: import('$lib/types/api').FigurineListItem | null;
			readonly sortedImages: import('$lib/types/api').FigurineImage[];
			readonly attributes: { kind: string; label: string; value: string }[];
			readonly visibleProcessSteps: import('$lib/types/api').ProcessStep[];
			readonly visibleRelatedItems: import('$lib/types/api').FigurineListItem[];
			readonly figurineSchedule: import('$lib/types/api').FigurineSchedule;
			readonly activeImageIndex: number;
			readonly currentImage: import('$lib/types/api').FigurineImage | undefined;
			readonly currentImageFit: 'cover' | 'contain';
			readonly imageViewMode: 'fit' | 'detail';
			readonly isLensEnabled: boolean;
			readonly isRakingEnabled: boolean;
			readonly useDaguerreotype: boolean;
			readonly useRaking: boolean;
			readonly showRakingButton: boolean;
			readonly isSaved: boolean;
			readonly canOpenLightbox: boolean;
			readonly bleedDir: 'prev' | 'next' | null;
			readonly lastBleed: { dir: 'prev' | 'next'; img: string } | null;
			readonly plateStyle: string;
			readonly viewTransitionName: string;
			readonly isCandleLit: boolean;
			readonly hasHistorySection: boolean;
			readonly hasMakingSection: boolean;
			readonly hasVideoSection: boolean;
			readonly hasWorkStorySection: boolean;
			readonly hasAttributesSection: boolean;
			readonly hasScheduleSection: boolean;
			readonly hasFactsSection: boolean;
			readonly hasPersonalRecord: boolean;
			readonly hasClaimRecords: boolean;
			readonly hasClaimLookupState: boolean;
			readonly canShowPersonalRecord: boolean;
			readonly hasBecoming: boolean;
			readonly showMirrorLink: boolean;
			readonly becomingBefore: string;
			readonly becomingAfter: string;
			readonly firstStep: import('$lib/types/api').ProcessStep | undefined;
			readonly lastStep: import('$lib/types/api').ProcessStep | undefined;
			readonly queueJoin: { token: string; position: number } | null;
			readonly notifyJoin: string | null;
			readonly hasActiveShowing: boolean;
			readonly nextAvailableDate: Date | null;
			readonly scheduleLoadFailed: boolean;
			readonly isGrimoireOpen: boolean;
			readonly displayConfig: import('$lib/types/api').DisplayConfig | null;
			readonly cs: import('$lib/stores/figurine-claims.svelte').FigurineClaimsStore;
			readonly analyticsClient: ReturnType<typeof import('$lib/analytics').createFigurineAnalytics> | null;
			readonly statusUi: {
				label: string; title: string; note: string;
				mobileSubtitle: string; mobileCtaLabel: string;
				mobileIcon: string; defaultIntent: string;
			};
			selectImage(index: number): void;
			openLightbox(index: number): void;
			toggleSaved(): void;
			toggleLens(): void;
			toggleRaking(): void;
			setImageViewMode(mode: 'fit' | 'detail'): void;
			openRequestModal(intent?: string): void;
			openGrimoire(): void;
			closeGrimoire(): void;
			handlePersonalRecordToggle(e: Event): void;
			openClaimLookup(): void;
			closeClaimLookup(): void;
			armPageTurn(e: MouseEvent, direction: 'forward' | 'backward'): void;
			setGalleryEl(el: HTMLElement | undefined): void;
			resolveUrl(path: string | undefined | null): string;
			imageTypeLabel(type: string | undefined | null): string;
			imageRoleNote(type: string | undefined | null): string;
			processStepLabel(type: string | undefined | null): string;
			fmtDate(ds: string): string;
			lookupStatusLabel(s: string): string;
			toRoman(n: number): string;
			hasText(value: string | null | undefined): value is string;
			statusLabel(status: import('$lib/types/api').FigurineStatus): string;
		}
	}

	// Build-time env consumed by the web (prerender) build.
	// VITE_BUILD_TARGET=web → enable SSR/prerender for public routes.
	// VITE_API_BASE        → absolute API origin used during prerender (no localStorage in Node).
	interface ImportMetaEnv {
		readonly VITE_API_BASE?: string;
		readonly VITE_BUILD_TARGET?: string;
		// Brand name baked in at build. Falls back to 'Gotiga'; admin copy override wins at runtime.
		readonly VITE_BRAND_NAME?: string;
	}
}

export {};
