// See https://svelte.dev/docs/kit/types#app.d.ts
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}

	// Build-time env consumed by the web (prerender) build.
	// VITE_BUILD_TARGET=web → enable SSR/prerender for public routes.
	// VITE_API_BASE        → absolute API origin used during prerender (no localStorage in Node).
	interface ImportMetaEnv {
		readonly VITE_API_BASE?: string;
		readonly VITE_BUILD_TARGET?: string;
	}
}

export {};
