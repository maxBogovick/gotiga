<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '$lib/api';
    import type { CutoutStatus } from '$lib/types/api';

    // Вырезание фигурок (CLAUDE.md § 21) живёт в отдельной программе на машине
    // автора: модель стоит там, а не на сервере сайта. Эта плита только
    // запускает её и показывает её страницу рамкой. На рабочем сервере ручек
    // нет (404) — тогда блока нет вовсе, а не «есть, но не работает».

    let status = $state<CutoutStatus | null>(null);
    let available = $state(false);
    let asking = $state(false);
    // Страница программы после каждого запуска грузится заново: иначе рамка,
    // открытая до выключения программы, осталась бы с ошибкой «не отвечает».
    let frameKey = $state(0);

    const RUNNING_EVERY_MS = 20_000;
    const STARTING_EVERY_MS = 2_000;

    async function refresh() {
        try {
            const next = await api.getCutoutStatus();
            if (next.running && !status?.running) frameKey += 1;
            status = next;
            available = true;
        } catch {
            // Первый ответ не пришёл — это не локальный сервер. Позже сбой сети
            // блок не убирает: он только что работал.
            if (!available) status = null;
        }
    }

    async function start() {
        asking = true;
        try {
            status = await api.startCutout();
        } catch (e) {
            status = {
                installed: true,
                running: false,
                url: null,
                starting: false,
                error: e instanceof Error ? e.message : String(e),
            };
        } finally {
            asking = false;
        }
    }

    onMount(() => {
        let timer: ReturnType<typeof setTimeout>;
        let gone = false;
        // Опрос нужен дважды: пока программа поднимается — чтобы показать окно
        // сразу; пока работает — чтобы увидеть, что она выключилась, и чтобы
        // сама она не выключалась по бездействию, пока админка открыта.
        const loop = async () => {
            await refresh();
            if (gone || !available) return;
            timer = setTimeout(loop, status?.starting ? STARTING_EVERY_MS : RUNNING_EVERY_MS);
        };
        loop();
        return () => {
            gone = true;
            clearTimeout(timer);
        };
    });
</script>

{#if available && status}
    <div class="border border-[#34251c]/15 px-4 py-3">
        <div class="flex flex-wrap items-center justify-between gap-2 mb-2">
            <div class="text-xs uppercase tracking-wide text-[#34251c]">Cut out figurines</div>
            {#if status.running && status.url}
                <a
                    href={status.url}
                    target="_blank"
                    rel="noopener"
                    class="text-[10px] uppercase tracking-wide text-[#5f4636] underline"
                >Open in a separate tab</a>
            {/if}
        </div>

        {#if status.running && status.url}
            {#key frameKey}
                <iframe
                    src={status.url}
                    title="Cut out figurines"
                    allow="clipboard-read; clipboard-write"
                    class="w-full h-[78vh] border border-[#34251c]/15 bg-[#f8f1e7]"
                ></iframe>
            {/key}
        {:else if !status.installed}
            <p class="text-xs text-[#5f4636]">
                The program is not found in <code>tools/cutout</code>.
            </p>
        {:else}
            <p class="text-xs text-[#5f4636] mb-3">
                Removes the background from photos with a model installed on this
                computer. It is available only in the local version of the site.
            </p>
            <button
                onclick={start}
                disabled={asking || status.starting}
                class="btn-gothic text-[10px] disabled:opacity-70"
            >{status.starting ? 'Starting…' : 'Start'}</button>
            {#if status.starting}
                <p class="text-[10px] text-[#5f4636] mt-2">
                    The first start installs the dependencies and takes a minute or two.
                </p>
            {/if}
            {#if status.error}
                <pre class="mt-3 border border-red-700/30 bg-red-50 px-3 py-2 text-[10px] text-red-800 whitespace-pre-wrap">{status.error}</pre>
            {/if}
        {/if}
    </div>
{/if}
