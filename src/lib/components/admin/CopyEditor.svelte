<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '$lib/api';
    import { t, setCopyOverrides } from '$lib/i18n';
    import { en } from '$lib/i18n/en';
    import { ru } from '$lib/i18n/ru';
    import { enAdmin } from '$lib/i18n/en.admin';
    import { ruAdmin } from '$lib/i18n/ru.admin';
    import type { TranslationKey, Lang } from '$lib/i18n';
    import type { CopyOverrides } from '$lib/types/api';

    let activeLang = $state<Lang>('en');
    let search = $state('');
    let saving = $state(false);
    let saved = $state(false);
    let error = $state('');
    let overridesDraft = $state<CopyOverrides>({ en: {}, ru: {} });
    let showModifiedOnly = $state(false);

    const staticDicts: Record<Lang, Record<string, string>> = {
        en: { ...en, ...enAdmin },
        ru: { ...ru, ...ruAdmin },
    };

    onMount(async () => {
        try {
            const remote = await api.getCopyOverrides();
            overridesDraft = { en: { ...remote.en }, ru: { ...remote.ru } };
        } catch {
            overridesDraft = { en: {}, ru: {} };
        }
    });

    let allKeys = $derived(Object.keys(staticDicts.en) as TranslationKey[]);

    let filteredKeys = $derived((() => {
        const q = search.trim().toLowerCase();
        const langOverrides = overridesDraft[activeLang] ?? {};
        return allKeys.filter(key => {
            if (showModifiedOnly && !langOverrides[key]) return false;
            if (!q) return true;
            const defaultVal = staticDicts[activeLang][key] ?? '';
            return key.toLowerCase().includes(q) || defaultVal.toLowerCase().includes(q);
        });
    })());

    function setOverride(key: string, value: string) {
        const defaultVal = staticDicts[activeLang][key] ?? '';
        const updated = { ...overridesDraft[activeLang] };
        if (value === '' || value === defaultVal) {
            delete updated[key];
        } else {
            updated[key] = value;
        }
        overridesDraft = { ...overridesDraft, [activeLang]: updated };
    }

    function clearOverride(key: string) {
        const updated = { ...overridesDraft[activeLang] };
        delete updated[key];
        overridesDraft = { ...overridesDraft, [activeLang]: updated };
    }

    function getCurrentValue(key: string): string {
        return overridesDraft[activeLang][key] ?? staticDicts[activeLang][key] ?? '';
    }

    function isModified(key: string): boolean {
        return key in overridesDraft[activeLang];
    }

    async function handleSave() {
        saving = true; error = ''; saved = false;
        try {
            await api.saveCopyOverrides(overridesDraft);
            setCopyOverrides(overridesDraft as Record<Lang, Record<string, string>>);
            saved = true;
            setTimeout(() => saved = false, 2500);
        } catch {
            error = $t('adminCopyError');
        } finally {
            saving = false;
        }
    }

    let modifiedCount = $derived(Object.keys(overridesDraft[activeLang] ?? {}).length);
</script>

<div class="h-full flex flex-col overflow-hidden">
    <!-- Header -->
    <div class="flex items-center justify-between shrink-0 mb-4">
        <div>
            <h2 class="text-xs uppercase tracking-widest text-[#5f4636]">{$t('adminCopyTitle')}</h2>
            {#if modifiedCount > 0}
                <p class="text-[10px] text-[#c65f3c] mt-0.5">{modifiedCount} {$t('adminCopyModified')}</p>
            {/if}
        </div>
        <button onclick={handleSave} disabled={saving}
            class="btn-gothic text-[10px] {saved ? 'text-green-700' : ''}">
            {#if saving}…{:else if saved}{$t('adminCopySaved')}{:else}{$t('adminCopySave')}{/if}
        </button>
    </div>

    {#if error}
        <p class="text-red-700 text-[10px] mb-2">{error}</p>
    {/if}

    <!-- Controls -->
    <div class="flex gap-2 shrink-0 mb-3">
        <!-- Language toggle -->
        <div class="flex border border-[#d8c6b1] text-[10px] uppercase tracking-wider overflow-hidden">
            {#each (['en','ru'] as Lang[]) as lang}
                <button
                    onclick={() => activeLang = lang}
                    class="px-3 py-1 transition-colors {activeLang === lang ? 'bg-[#34251c] text-[#f8f1e7]' : 'text-[#5f4636] hover:bg-[#34251c]/5'}"
                >{lang.toUpperCase()}</button>
            {/each}
        </div>

        <!-- Search -->
        <input
            type="search"
            bind:value={search}
            placeholder={$t('adminCopySearch')}
            class="flex-1 border border-[#d8c6b1] bg-[#fff9f0] text-[#34251c] text-xs px-2 py-1 focus:outline-none focus:border-[#c65f3c]"
        />

        <!-- Modified filter -->
        <label class="flex items-center gap-1.5 text-[10px] text-[#5f4636] cursor-pointer whitespace-nowrap">
            <input type="checkbox" bind:checked={showModifiedOnly} class="accent-[#c65f3c]" />
            {$t('adminCopyModified')}
        </label>
    </div>

    <!-- Keys list -->
    <div class="flex-1 overflow-y-auto border border-[#d8c6b1]/50">
        {#if filteredKeys.length === 0}
            <div class="flex items-center justify-center h-20 text-[11px] text-[#5f4636]/50">
                {$t('adminCopyNoResults')}
            </div>
        {:else}
            <table class="w-full text-xs border-collapse">
                <colgroup>
                    <col style="width: 30%" />
                    <col style="width: 70%" />
                </colgroup>
                <tbody>
                {#each filteredKeys as key}
                    {@const modified = isModified(key)}
                    {@const currentVal = getCurrentValue(key)}
                    {@const defaultVal = staticDicts[activeLang][key] ?? ''}
                    <tr class="border-b border-[#d8c6b1]/30 {modified ? 'bg-[#c65f3c]/5' : ''} hover:bg-[#34251c]/3">
                        <td class="px-2 py-1.5 align-top">
                            <div class="flex items-start gap-1">
                                <span class="font-mono text-[10px] text-[#5f4636]/70 break-all leading-relaxed">{key}</span>
                                {#if modified}
                                    <span class="shrink-0 mt-0.5 inline-block w-1.5 h-1.5 rounded-full bg-[#c65f3c]"></span>
                                {/if}
                            </div>
                            {#if modified}
                                <p class="text-[9px] text-[#5f4636]/40 mt-0.5 leading-tight italic">{defaultVal}</p>
                            {/if}
                        </td>
                        <td class="px-2 py-1.5 align-top">
                            <div class="flex gap-1">
                                <input
                                    type="text"
                                    value={currentVal}
                                    oninput={(e) => setOverride(key, (e.target as HTMLInputElement).value)}
                                    class="flex-1 min-w-0 border border-[#d8c6b1]/60 bg-transparent text-[#34251c] text-[11px] px-1.5 py-0.5
                                        focus:outline-none focus:border-[#c65f3c] {modified ? 'border-[#c65f3c]/40' : ''}"
                                />
                                {#if modified}
                                    <button
                                        onclick={() => clearOverride(key)}
                                        class="shrink-0 text-[9px] text-[#5f4636]/50 hover:text-[#c65f3c] px-1 border border-[#d8c6b1]/60"
                                        title={$t('adminCopyClear')}
                                    >✕</button>
                                {/if}
                            </div>
                        </td>
                    </tr>
                {/each}
                </tbody>
            </table>
        {/if}
    </div>

    <p class="shrink-0 text-[9px] text-[#5f4636]/40 mt-2">
        {filteredKeys.length} / {allKeys.length} keys · {activeLang.toUpperCase()}
    </p>
</div>
