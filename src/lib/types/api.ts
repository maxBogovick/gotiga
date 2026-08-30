// src/lib/types/api.ts

export type FigurineStatus = 'available' | 'sold' | 'reserved' | 'in_progress';

export interface FigurineListItem {
    id: string;
    /** Transliterated URL slug; null for works not yet re-saved. Prefer over `id`
     *  when building `/figurines/…` links (use `figurineHref`). */
    slug?: string | null;
    /** True when the slug was hand-typed by an admin (differs from the name-derived
     *  auto slug); false/absent when auto-generated. Drives the «Work addresses» badge. */
    slugManual?: boolean;
    name: string;
    status: FigurineStatus;
    /** The work's one-line note, carried on the list payload. The home gallery used to
     *  fetch a full `getFigurine(id)` per visible work just to read this one string. */
    shortText?: string | null;
    /** 420px thumbnail — sized for the archive's dense grid. */
    faceImageUrl: string | null;
    /** Second-angle image for the home gallery's hover reveal; null/absent
     *  when the piece has no dedicated "detail" image. */
    detailImageUrl?: string | null;
    /** The same two images at preview size (1800px). Use these wherever a list
     *  item is rendered large — the home hero, the home reel plates — or the
     *  420px thumb gets upscaled 2-3x and goes soft. Null when the work has no
     *  such image; fall back to faceImageUrl. */
    faceImageLargeUrl?: string | null;
    detailImageLargeUrl?: string | null;
    year?: number | null;
    sortOrder?: number;
    series?: string | null;
    technique?: string | null;
    material?: string | null;
    dimensions?: string | null;
    isFeatured?: boolean;
    createdAt?: string | null;
    /** When the work was last edited (status, photo, text). Drives the home
     *  "since your visit" ledger's "refreshed" signal. */
    updatedAt?: string | null;
    thumbUrl?: string | null;
    /**
     * "Keyhole" reveal of the face image on the card: focus point and radius,
     * all normalised 0..1. Null/undefined → centred focus + renderer default
     * radius. Set per image in the admin figurine form.
     */
    focalX?: number | null;
    focalY?: number | null;
    revealRadius?: number | null;
    /** Per-image darkness override (0..1); null → global keyhole darkness. */
    darkness?: number | null;
    /**
     * "The house wakes" — daily showing window, minutes from midnight (0..1439),
     * read against the visitor's LOCAL clock. Both null → always open. When the
     * window is closed the card shows a carved sealed door instead of the work,
     * and is not enterable. `openUntilMin < openFromMin` wraps past midnight.
     */
    openFromMin?: number | null;
    openUntilMin?: number | null;
    /**
     * Optional showing room this work belongs to. When set, the room's window is
     * used instead of openFromMin/openUntilMin (mutually exclusive). null → own window.
     */
    showingRoomId?: string | null;
    /**
     * "First look" early-release window (ISO-8601). While now < this, the work is
     * held out of the public archive and shown only on the book-holders' shelf.
     * null → ordinary public work.
     */
    firstLookUntil?: string | null;
    /** "House Favorite" — rare, loud badge for pieces past a private mark-score
     *  threshold. Never a number, just a boolean (see backend HOUSE_FAVORITE_THRESHOLD). */
    houseFavorite?: boolean;
}

/** A named, shared showing window several works can point at (e.g. "Night hall"). */
export interface ShowingRoom {
    id: string;
    name: string;
    openFromMin: number;
    openUntilMin: number;
    /** Allowed weekdays bitmask (bit0=Mon … bit6=Sun); null → every day. */
    openDaysMask?: number | null;
    /** "MM-DD" — open every year on that date; null → unused. */
    openMonthDay?: string | null;
    /** One-off inclusive date range "YYYY-MM-DD"; null → unused. */
    openDateFrom?: string | null;
    openDateUntil?: string | null;
}

export interface FigurineImage {
    id: string;
    imageType: 'face' | 'detail' | 'full';
    url: string;
    originalUrl: string | null;
    thumbUrl: string | null;
    altText: string | null;
    /**
     * Precomputed monocular depth map (grayscale), used by LivingDaguerreotype
     * for 2.5D parallax. Optional — when absent the renderer derives depth from
     * the colour image's luminance instead. Populated by an offline pipeline.
     */
    depthUrl?: string | null;
    /**
     * Optional 0..1 multiplier for the 2.5D parallax. Null/undefined means the
     * renderer default is used, preserving old catalogue behaviour.
     */
    parallaxIntensity?: number | null;
    /**
     * "Keyhole" reveal focus point + radius (all normalised 0..1) for the card
     * teaser. Null → centred focus + default radius. Edited in the admin form.
     */
    focalX?: number | null;
    focalY?: number | null;
    revealRadius?: number | null;
    /** Per-image darkness override (0..1); null → global keyhole darkness. */
    darkness?: number | null;
}

export interface DepthGenItem {
    imageId: string;
    status: 'done' | 'skip' | 'fail';
    detail: string | null;
}

export interface DepthGenSummary {
    generated: number;
    skipped: number;
    failed: number;
    results: DepthGenItem[];
}

/** One ranked result of a hybrid search ("Хранитель"). */
export interface SemanticHit {
    id: string;
    /** Reciprocal Rank Fusion score; higher is closer. Not cosine. */
    score: number;
}

/** Result of an admin re-index of figurine search embeddings. */
export interface EmbedIndexSummary {
    total: number;
    indexed: number;
    skipped: number;
    failed: number;
}

export interface BulkOpResult {
    affected: number;
}

export interface ProcessStep {
    id: string;
    stepType: 'sketch' | 'prototype' | 'modeling' | 'painting' | 'finish';
    description: string | null;
    imageUrl: string;
}

export interface Figurine {
    id: string;
    /** Transliterated URL slug; null for works not yet re-saved. Canonical URL is
     *  `/figurines/{slug ?? id}`. */
    slug?: string | null;
    name: string;
    shortText: string | null;
    fullDescription: string | null;
    dimensions: string | null;
    material: string | null;
    technique: string | null;
    year: number | null;
    passportNumber: string | null;
    edition: string | null;
    createdPeriod: string | null;
    careInstructions: string | null;
    provenanceNote: string | null;
    authenticityNote: string | null;
    includedItems: string | null;
    ambiencePath: string | null;
    videoUrl: string | null;
    secretText: string | null;
    status: FigurineStatus;
    sortOrder: number;
    isVisible: boolean;
    isFeatured: boolean;
    series?: string | null;
    /** "The house wakes" — showing window, minutes from midnight; both null → always open. */
    openFromMin?: number | null;
    openUntilMin?: number | null;
    /** Showing room this work belongs to; null → uses its own window. */
    showingRoomId?: string | null;
    images: FigurineImage[];
    processSteps: ProcessStep[];
    relatedItems: FigurineListItem[];
    displayLayout?: 'specimen' | 'showcase' | 'codex' | 'diptych' | 'broadside' | null;
    /** JSON string — parsed by the frontend into DisplayConfig. */
    displayConfig?: string | null;
    /** JSON string — parsed into CatalogLists (Features / Perfect for).
     *  Null / omitted → every built-in line is on, no custom lines. */
    catalogLists?: string | null;
    /** "First look" early-release window (ISO-8601); null → ordinary public work. */
    firstLookUntil?: string | null;
    /** True once this piece crosses a private mark-count threshold. The only
     *  public trace of the marks-of-attention system — never a raw number. */
    noticedByOthers?: boolean;
    /** "House Favorite" — the loud, rare second tier above noticedByOthers. */
    houseFavorite?: boolean;
}

export type DisplayConfigBackground =
    | 'parchment' | 'aged' | 'dark' | 'linen' | 'slate' | 'custom';

export interface BlockStyle {
    color?: string;
    background?: string;
    fontSize?: 'sm' | 'base' | 'lg' | 'xl';
    /** Id from SITE_FONTS (e.g. 'garamond', 'cormorant'). */
    font?: string;
}

type _ContentBlockId = 'description' | 'making' | 'video' | 'showings' | 'related' | 'comments';
type _UpperZoneId = 'name' | 'shortText' | 'attrs' | 'eyebrow';
type _StyleableZoneId = _UpperZoneId | _ContentBlockId;

export interface DisplayConfigPreset {
    id: string;
    name: string;
    config: DisplayConfig;
    savedAt: string;
}

export interface DisplayConfig {
    background?: {
        preset?: DisplayConfigBackground;
        customColor?: string;
        textureUrl?: string;
    };
    blockOrder?: Array<_ContentBlockId>;
    hiddenBlocks?: Array<_ContentBlockId>;
    blockStyles?: Partial<Record<_StyleableZoneId, BlockStyle>>;
}

export interface AuthorText {
    id: string;
    content: string;
}

export interface WorkshopItem {
    id: string;
    content: string;
    caption: string | null;
    imageUrl: string | null;
}

export interface ServerRelease {
    id: string;
    version: number;
    createdAt: string;
    description: string | null;
    isActive: boolean;
}

export interface AppSettings {
    serverUrl: string;
    apiKey: string;
}

export interface HomeContent {
    title: string | null;
    kicker: string | null;
    lead: string | null;
    heroFigurineId: string | null;
    heroCaptionTitle: string | null;
    heroCaptionMeta: string | null;
    heroCaptionCta: string | null;
    heroMode: 'auto' | 'showcase' | 'release' | null;
    /** Admin-pinned figurine for the "exhibit of the day" vitrine — independent of heroFigurineId. null → daily rotation. */
    vitrineFigurineId: string | null;
}

export type AvatarShape = 'circle' | 'square';

export interface AuthorProfile {
    name: string;
    tagline: string | null;
    bio: string | null;
    photoUrl: string | null;
    /** Portrait for the site-header avatar — distinct from `photoUrl` (used
     *  by the bio/author page). Falls back to a static asset when unset. */
    heroPhotoUrl: string | null;
    instagram: string | null;
    telegram: string | null;
    vk: string | null;
    email: string | null;
    website: string | null;
    artstation: string | null;
    pinterest: string | null;
    youtube: string | null;
    /** Header avatar frame styling — all optional, admin-editable. */
    avatarShape: AvatarShape | null;
    /** Corner radius in px, used only when avatarShape is 'square'. */
    avatarRadius: number | null;
    /** Ring thickness in px. */
    avatarBorderWidth: number | null;
    avatarBorderColor: string | null;
    /** Matting colour shown behind the photo, inside the ring. */
    avatarBg: string | null;
}

export type OrderMode = 'request' | 'question' | 'notify' | 'reserve';
export type OrderStatus = 'new' | 'seen' | 'replied';
export type ReserveStatus = 'requested' | 'reviewing' | 'terms_sent' | 'confirmed' | 'declined' | 'expired';

export interface OrderRequest {
    figurineId: string;
    figurineName: string;
    requesterName: string;
    requesterEmail: string;
    requesterPhone: string | null;
    message: string | null;
    mode: OrderMode;
    /** Self-certified "I am 16 or older" checkbox. */
    ageConfirmed: boolean;
}

export interface Order {
    id: string;
    figurineId: string;
    figurineName: string;
    requesterName: string;
    requesterEmail: string;
    requesterPhone: string | null;
    message: string | null;
    mode: OrderMode;
    status: OrderStatus;
    adminNotes: string | null;
    reserveStatus: ReserveStatus | null;
    reserveExpiresAt: string | null;
    adminTermsNote: string | null;
    invoiceNote: string | null;
    certificateToken: string | null;
    certificateNumber: string | null;
    certificateIssuedAt: string | null;
    certificateRevokedAt: string | null;
    createdAt: string;
}

export interface OrdersPage {
    items: Order[];
    total: number;
    newCount: number;
    page: number;
    perPage: number;
}

export type AdminLogLevel = 'TRACE' | 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';

export interface AdminLogEntry {
    id: number;
    ts: string;
    level: AdminLogLevel | string;
    target: string;
    message: string;
    requestId: string | null;
    method: string | null;
    route: string | null;
    status: number | null;
    latencyMs: number | null;
    fields: Record<string, unknown>;
}

export interface AdminLogsPage {
    items: AdminLogEntry[];
    nextBeforeId: number | null;
    nextOffset: number | null;
    droppedTotal: number;
}

export type AdminLogsSortBy = 'time' | 'level' | 'request' | 'route' | 'status' | 'latency' | 'message';
export type AdminLogsSortDir = 'asc' | 'desc';

export interface AdminLogsQuery {
    beforeId?: number | null;
    offset?: number | null;
    sortBy?: AdminLogsSortBy;
    sortDir?: AdminLogsSortDir;
    from?: string;
    to?: string;
    level?: string;
    requestId?: string;
    route?: string;
    method?: string;
    status?: number | null;
    statusClass?: number | null;
    minLatencyMs?: number | null;
    maxLatencyMs?: number | null;
    target?: string;
    q?: string;
    limit?: number;
}

export type AnalyticsSignal =
    | 'high_conversion'
    | 'attention_no_submissions'
    | 'low_visibility'
    | 'growing_interest'
    | 'low_data'
    | 'normal';

export interface AnalyticsSummary {
    views: number;
    uniqueVisitors: number;
    engagedViews: number;
    ctaClicks: number;
    submissions: number;
    conversionRate: number;
}

export interface AnalyticsDailyPoint {
    day: string;
    views: number;
    uniqueVisitors: number;
    engagedViews: number;
    ctaClicks: number;
    submissions: number;
}

export interface AnalyticsSourcePoint {
    source: string;
    views: number;
    uniqueVisitors: number;
}

export interface AnalyticsBreakdownPoint {
    key: string;
    views: number;
    uniqueVisitors: number;
}

/** One (day, country) cell from the permanent geo rollup — the geography
 * map's "one figurine" mode groups these by country for the choropleth, then
 * filters by country to list the actual dates a visit was recorded. */
export interface FigurineGeoDailyPoint {
    day: string;
    countryCode: string;
    views: number;
    uniqueVisitors: number;
}

export interface AnalyticsFunnel {
    views: number;
    engagedViews: number;
    ctaClicks: number;
    submissions: number;
}

/** One row of the starts -> submitted funnel for a single CTA family. `starts`
 * is client-side event counts (undercounts under DNT/bots/direct links);
 * `submitted` is authoritative (counted from the real orders/bookings/
 * waitlist/commissions tables) — conversion can legitimately read over 100%. */
export interface CtaFunnelStep {
    ctaType: 'request' | 'reserve' | 'booking' | 'waitlist' | 'commission';
    starts: number;
    submitted: number;
}

export interface AdminFigurineAnalyticsListItem {
    figurineId: string;
    name: string;
    status: FigurineStatus;
    /** Editorial grouping — same field the archive page filters by. */
    series?: string | null;
    faceUrl: string | null;
    signal: AnalyticsSignal;
    /** Week-over-week growth on its own, independent of `signal` — `signal`
     * is a single priority-ordered pick, so a work that's both growing and,
     * say, attention-worthy would only ever surface the higher-priority
     * badge. Use this to show growth regardless of which signal won. */
    isGrowing: boolean;
    topSource?: string | null;
    topCountry?: string | null;
    topDevice?: string | null;
    topBrowser?: string | null;
    /** Every country (ISO 3166-1 alpha-2) with at least one view in range —
     * not just the top one. Powers the Works table's country filter and the
     * geography map's country → works drilldown, both client-side. */
    countries: string[];
    views: number;
    uniqueVisitors: number;
    engagedViews: number;
    ctaClicks: number;
    submissions: number;
    conversionRate: number;
    /** Daily views, last 14 days ending at the query's `to`, zero-filled. */
    sparkline: number[];
}

export interface AdminFigurineAnalyticsListPage {
    items: AdminFigurineAnalyticsListItem[];
    total: number;
    summary: AnalyticsSummary;
    /** Same-length, immediately-preceding period for delta comparisons. */
    previousSummary: AnalyticsSummary;
    previousFrom: string;
    previousTo: string;
}

export interface AdminFigurineAnalyticsDetail {
    figurine: FigurineListItem;
    signal: AnalyticsSignal;
    summary: AnalyticsSummary;
    previousSummary: AnalyticsSummary;
    previousFrom: string;
    previousTo: string;
    daily: AnalyticsDailyPoint[];
    sources: AnalyticsSourcePoint[];
    countries: AnalyticsBreakdownPoint[];
    devices: AnalyticsBreakdownPoint[];
    browsers: AnalyticsBreakdownPoint[];
    referrers: AnalyticsBreakdownPoint[];
    utmSources: AnalyticsBreakdownPoint[];
    visitorCohorts: AnalyticsBreakdownPoint[];
    languages: AnalyticsBreakdownPoint[];
    internalSources: AnalyticsBreakdownPoint[];
    funnel: AnalyticsFunnel;
    ctaFunnel: CtaFunnelStep[];
    /** Median ms engaged with the card. Null when there are no qualifying
     * events in range — never coerced to 0. */
    medianDurationMs: number | null;
    /** Median scroll depth (0-100) at engagement. Same null rule. */
    medianScrollDepth: number | null;
    /** Earliest date the raw-event-derived fields above (medians, the
     * breakdowns) actually have data for — later than `from` when the
     * selected range reaches past raw-event retention. */
    rawDataFrom: string;
}

export interface AdminAnalyticsQuery {
    from?: string;
    to?: string;
    sort?: string;
    dir?: 'asc' | 'desc';
}

/** One-off admin action: re-run the daily aggregation over a historical
 * range (e.g. after a fix to the aggregation query itself — the automatic
 * hot-window refresh only ever recomputes yesterday+today). No UI trigger
 * yet; call via `api.backfillAnalytics()` (e.g. from devtools) after
 * deploying a fix that needs one. */
export interface BackfillAnalyticsRequest {
    from?: string;
    to?: string;
}

export interface BackfillAnalyticsResponse {
    from: string;
    to: string;
}

/** Site-wide traffic overview (all figurines combined) — the "pulse of the
 * house" screen. */
export interface AdminAnalyticsOverview {
    from: string;
    to: string;
    previousFrom: string;
    previousTo: string;
    summary: AnalyticsSummary;
    previousSummary: AnalyticsSummary;
    daily: AnalyticsDailyPoint[];
    sources: AnalyticsSourcePoint[];
    /** Site-wide views by country (every page), for the geography map. */
    geo: AnalyticsBreakdownPoint[];
}

/** Site → works → /commission → started form → submitted. The first four
 * counts are distinct visitors from raw events (retention-bound — see
 * rawDataFrom); `submitted` is exact. */
export interface CommissionFunnel {
    from: string;
    to: string;
    rawDataFrom: string;
    visited: number;
    viewedWorks: number;
    openedCommissionPage: number;
    startedForm: number;
    submitted: number;
}

export interface AnalyticsAnnotation {
    id: string;
    day: string;
    label: string;
    createdAt: string;
}

export interface CreateAnnotationRequest {
    day: string;
    label: string;
}

/** Daily marks/subscribers/comments — full history, no retention pruning. */
export interface LifeOfHouseDailyPoint {
    day: string;
    marks: number;
    subscribers: number;
    comments: number;
}

export interface LifeOfHouseTrend {
    from: string;
    to: string;
    daily: LifeOfHouseDailyPoint[];
    marksTotal: number;
    subscribersTotal: number;
    commentsTotal: number;
    previousMarksTotal: number;
    previousSubscribersTotal: number;
    previousCommentsTotal: number;
}

export interface AnalyticsEventPayload {
    eventType: 'figurine_view' | 'figurine_engaged' | 'figurine_cta_click' | 'page_view' | 'page_engaged';
    /** Required for figurine_view/figurine_engaged/figurine_cta_click; absent
     * for site-wide page_view/page_engaged events. */
    figurineId?: string | null;
    path: string;
    referrer?: string | null;
    utmSource?: string | null;
    utmMedium?: string | null;
    utmCampaign?: string | null;
    durationMs?: number | null;
    scrollDepth?: number | null;
    /** Distinct work tiles seen during a home/archive visit (page_engaged only);
     * absent for gridless pages and all other event types. */
    worksSeen?: number | null;
    ctaType?: string | null;
    pageViewId?: string | null;
    clientTs?: string;
    lang?: string | null;
    /** Which on-site block a figurine-card click came from, e.g. "home_afisha". */
    internalSource?: string | null;
}

/** One generic page's engagement (home/archive/author/workshop/commission).
 * `views`/`uniqueVisitors` cover the full range; everything from `engagedEvents`
 * on is derived from raw `page_engaged` events and so only covers retention.
 * Medians are null when no events qualify; `medianWorksSeen`/`reachedWorksEvents`
 * are meaningful only for the grid pages (home/archive). */
export interface SitePageEngagement {
    pathGroup: string;
    views: number;
    uniqueVisitors: number;
    engagedEvents: number;
    /** Engaged visits shorter than the quick-exit threshold (near-bounces). */
    quickExitEvents: number;
    /** Engaged grid-page visits that saw at least one work tile. */
    reachedWorksEvents: number;
    medianDurationMs: number | null;
    medianScrollDepth: number | null;
    medianWorksSeen: number | null;
}

export interface SitePageEngagementResponse {
    from: string;
    to: string;
    /** Immediately-preceding equal-length period — the delta baseline. */
    previousFrom: string;
    previousTo: string;
    /** Earliest day the engagement figures actually reach (retention floor). */
    rawDataFrom: string;
    pages: SitePageEngagement[];
    /** Same shape as `pages`, for `previousFrom..previousTo`. */
    previousPages: SitePageEngagement[];
}

/** One anonymous visitor's day summary. "Visitor" is the daily-rotating,
 * pseudonymous `visitorHash` — no IP, no identity, cannot be followed across
 * days. Derived from raw events, so retention-bound. */
export interface AdminVisitorSession {
    visitorHash: string;
    day: string;
    firstSeen: string;
    lastSeen: string;
    eventCount: number;
    pageViews: number;
    figurineViews: number;
    ctaClicks: number;
    /** Distinct action buttons pressed during the visit — the visit's "trace";
     * empty when the visitor only browsed. */
    ctaTypes: string[];
    maxWorksSeen: number | null;
    maxScrollDepth: number | null;
    countryCode: string | null;
    deviceClass: string | null;
    browserFamily: string | null;
    lang: string | null;
    source: string | null;
}

export interface AdminVisitorSessionsPage {
    sessions: AdminVisitorSession[];
    total: number;
    from: string;
    to: string;
    rawDataFrom: string;
}

/** One event on an anonymous visitor's timeline. */
export interface AdminVisitorEvent {
    occurredAt: string;
    eventType: 'page_view' | 'page_engaged' | 'figurine_view' | 'figurine_engaged' | 'figurine_cta_click' | string;
    path: string;
    figurineId: string | null;
    figurineName: string | null;
    durationMs: number | null;
    scrollDepth: number | null;
    worksSeen: number | null;
    ctaType: string | null;
    source: string | null;
    internalSource: string | null;
}

// ── Commissions: a petition to the master to create a NEW figurine ──
export type CommissionStatus =
    | 'new' | 'reviewing' | 'accepted' | 'in_progress' | 'completed' | 'declined';

export interface CommissionRequest {
    requesterName?: string | null;
    requesterEmail: string;
    requesterPhone?: string | null;
    title?: string | null;
    description: string;
    sizeNote?: string | null;
    mood?: string | null;
    deadline?: string | null;
    budgetNote?: string | null;
    occasion?: string | null;
    sourceFigurineId?: string | null;
    similarKeepNote?: string | null;
    similarChangeNote?: string | null;
    similarTags?: string[];
    attachmentUrls?: AttachmentInput[];
    /** Honeypot — leave empty. */
    website?: string | null;
    /** UI language at submission ('ru' | 'en'). */
    lang?: string | null;
    /** Self-certified "I am 16 or older" checkbox. */
    ageConfirmed: boolean;
}

export interface EditCommissionRequest {
    title?: string | null;
    description: string;
    sizeNote?: string | null;
    mood?: string | null;
    deadline?: string | null;
    budgetNote?: string | null;
    occasion?: string | null;
    /** Full replacement set of references. Omit to leave attachments untouched. */
    attachmentUrls?: AttachmentInput[];
}

export interface CommissionDto {
    id: string;
    claimToken: string;
    requesterName: string;
    requesterEmail: string;
    requesterPhone: string | null;
    title: string;
    description: string;
    sizeNote: string | null;
    mood: string | null;
    deadline: string | null;
    budgetNote: string | null;
    occasion: string | null;
    sourceFigurineId: string | null;
    similarKeepNote: string | null;
    similarChangeNote: string | null;
    similarTags: string[];
    figurineId: string | null;
    status: CommissionStatus;
    adminNotes: string | null;
    createdAt: string;
    updatedAt: string;
    attachments: AttachmentDto[];
    threadId: string | null;
    /** Work has begun — petition can no longer be edited or deleted. */
    started: boolean;
    /** Certificate of authenticity, present once issued for a completed commission. */
    certificate: CollectorCertificateDto | null;
}

export interface CommissionCreatedResponse {
    id: string;
    claimToken: string;
}

export type LinkClaimKind = 'booking' | 'waitlist' | 'notify' | 'commission';
export type LinkClaimResult = 'linked' | 'email_mismatch' | 'already_linked' | 'not_found';

export interface LinkClaimResponse {
    result: LinkClaimResult;
    kind: LinkClaimKind | null;
    name: string | null;
}

export interface CommissionsPage {
    items: CommissionDto[];
    total: number;
    newCount: number;
    page: number;
    perPage: number;
}

export interface MediaUsage {
    path: string;
    label: string;
    entityType: string;
    entityId: string;
    field: string;
}

export interface MediaFile {
    path: string;
    url: string;
    mediaType: 'image' | 'video' | 'audio' | 'other';
    variant: string | null;
    sizeBytes: number;
    exists: boolean;
    usages: MediaUsage[];
}

export interface MediaInventory {
    files: MediaFile[];
    orphanCount: number;
    usedCount: number;
    totalSizeBytes: number;
}

export interface MediaCleanupReport {
    files: MediaFile[];
    totalSizeBytes: number;
}

export interface MediaReplaceResult {
    oldPath: string;
    newPath: string;
    updatedReferences: number;
    importedPaths: string[];
}

// ============================================================
// SHOWINGS & BOOKINGS
// ============================================================

export type ShowingType = 'exhibition' | 'private';
export type BookingStatus = 'pending' | 'confirmed' | 'rejected' | 'cancelled' | 'completed';

export interface ScheduleEntry {
    entryType: 'showing' | 'booking' | 'pending';
    title: string | null;
    showingType: ShowingType | null;
    venue: string | null;
    startsAt: string; // ISO date YYYY-MM-DD
    endsAt: string;
}

export interface FigurineSchedule {
    entries: ScheduleEntry[];
}

export interface ShowingDto {
    id: string;
    figurineId: string;
    title: string;
    showingType: ShowingType;
    startsAt: string;
    endsAt: string;
    venue: string | null;
    notes: string | null;
}

export interface BookingDto {
    id: string;
    figurineId: string;
    figurineName: string;
    requesterName: string;
    requesterEmail: string;
    requesterPhone: string | null;
    purpose: string | null;
    displayType: string | null;
    venue: string | null;
    curatorConditions: string | null;
    startsAt: string;
    endsAt: string;
    status: BookingStatus;
    adminNotes: string | null;
    createdAt: string;
}

export interface BookingsPage {
    items: BookingDto[];
    total: number;
    pendingCount: number;
    page: number;
    perPage: number;
}

export interface BookingCreatedResponse {
    cancelToken: string;
}

export interface BookingCancelInfo {
    figurineName: string;
    figurineId: string;
    startsAt: string;
    endsAt: string;
    status: BookingStatus;
    adminNotes: string | null;
    curatorConditions: string | null;
}

/** A single quiet wax-seal gesture a visitor can leave on a figurine, in one of 3
 *  private tones. Not a rating — no numeric value or tone is ever shown on the
 *  public site; only whether *this* visitor has marked *this* piece. */
export type MarkTone = 'touched' | 'mesmerized' | 'desired';

export interface LikeToggleResponse {
    liked: boolean;
    likeCount: number;
}

export interface MarkToggleResponse {
    marked: boolean;
    tone: MarkTone | null;
}

/** Admin-only ranking row — never rendered on the public site. `desired` (closest
 *  to commission intent) is weighted highest in `weightedScore`. */
export interface AdminFigurineMarkStat {
    figurineId: string;
    figurineName: string;
    status: FigurineStatus;
    isVisible: boolean;
    markCount: number;
    likeCount: number;
    touchedCount: number;
    mesmerizedCount: number;
    desiredCount: number;
    weightedScore: number;
    lastMarkedAt: string | null;
}

/** Admin pin/exclude config for the public "noticed by guests" home shelf —
 *  see NoticedByGuests.svelte for the hybrid resolution (pins first, then
 *  auto-fill from the private mark ranking). */
export interface NoticedByGuestsSettings {
    pinnedIds: string[];
    excludedIds: string[];
}

export interface CreateBookingRequest {
    figurineId: string;
    figurineName: string;
    requesterName: string;
    requesterEmail: string;
    requesterPhone: string | null;
    purpose: string | null;
    displayType: string | null;
    venue: string | null;
    startsAt: string;
    endsAt: string;
    /** Self-certified "I am 16 or older" checkbox. */
    ageConfirmed: boolean;
}

export interface SaveShowingRequest {
    id?: string;
    figurineId: string;
    title: string;
    showingType: ShowingType;
    startsAt: string;
    endsAt: string;
    venue: string | null;
    notes: string | null;
}

// ── User auth ────────────────────────────────────────────────

export interface UserDto {
    id: string;
    email: string;
    displayName: string;
    avatarUrl?: string | null;
    createdAt?: string;
}

export interface ChallengeIconDto {
    token: string;
    iconId: string;
}

export interface ChallengeStepDto {
    category: string;
    icons: ChallengeIconDto[];
}

export interface LoginChallengeResponse {
    challengeId: string;
    steps: ChallengeStepDto[];
}

export interface LoginVerifyResponse {
    sessionToken: string;
    user: UserDto;
}

export interface UserBookingDto {
    id: string;
    figurineId: string;
    figurineName: string;
    startsAt: string;
    endsAt: string;
    status: BookingStatus;
    createdAt: string;
    cancelToken: string;
    displayType: string | null;
    venue: string | null;
    curatorConditions: string | null;
}

export interface UserOrderDto {
    id: string;
    figurineId: string;
    figurineName: string;
    mode: OrderMode;
    status: OrderStatus;
    createdAt: string;
    adminNotes: string | null;
    reserveStatus: ReserveStatus | null;
    reserveExpiresAt: string | null;
    adminTermsNote: string | null;
    invoiceNote: string | null;
    certificate: CollectorCertificateDto | null;
}

export interface CollectorCertificateDto {
    token: string;
    certificateNumber: string;
    figurineId: string;
    figurineName: string;
    orderId: string;
    issuedAt: string;
    revokedAt: string | null;
}

export interface PublicCertificateDto {
    token: string;
    certificateNumber: string;
    figurineId: string;
    figurineName: string;
    issuedAt: string;
    revoked: boolean;
}

// ── Admin user management ────────────────────────────────────

export interface AdminUserListItem {
    id: string;
    email: string;
    displayName: string;
    adminNotes: string | null;
    isBlocked: boolean;
    createdAt: string;
    bookingCount: number;
    orderCount: number;
}

export interface AdminSessionDto {
    id: string;
    createdAt: string;
    expiresAt: string;
    isActive: boolean;
    ip: string | null;
    countryCode: string | null;
    city: string | null;
}

export interface MessageThreadDto {
    id: string;
    category: 'booking' | 'waitlist' | 'order' | 'commission' | 'general' | 'system';
    referenceId: string | null;
    subject: string;
    status: 'open' | 'resolved';
    unread: number;
    lastMessageAt: string;
    createdAt: string;
    preview: string | null;
}

export interface AttachmentDto {
    id: string;
    url: string;
    thumbUrl: string | null;
}

/** One uploaded reference, echoed back to the server after an upload. */
export interface AttachmentInput {
    url: string;
    thumbUrl?: string | null;
}

export interface ThreadMessageDto {
    id: string;
    threadId: string;
    fromAdmin: boolean;
    body: string;
    readAt: string | null;
    createdAt: string;
    attachments?: AttachmentDto[];
}

export interface ThreadDetailDto {
    thread: MessageThreadDto;
    messages: ThreadMessageDto[];
    user: { id: string; displayName: string; email: string } | null;
}

export interface AdminUserDetail {
    id: string;
    email: string;
    displayName: string;
    adminNotes: string | null;
    isBlocked: boolean;
    createdAt: string;
    signupIp: string | null;
    signupCountryCode: string | null;
    signupCity: string | null;
    lastResetIp: string | null;
    lastResetCountryCode: string | null;
    lastResetCity: string | null;
    lastResetAt: string | null;
    lastResetRequestIp: string | null;
    lastResetRequestCountryCode: string | null;
    lastResetRequestCity: string | null;
    lastResetRequestAt: string | null;
    bookings: UserBookingDto[];
    orders: UserOrderDto[];
    sessions: AdminSessionDto[];
    recentFailures: number;
    messages: MessageThreadDto[];
}

export interface ResetTokenResponse {
    token: string;
    expiresAt: string;
}

export interface AdminUsersPage {
    items: AdminUserListItem[];
    total: number;
    page: number;
    perPage: number;
}

// ── Comments ─────────────────────────────────────────────────

export interface CommentDto {
    id: string;
    authorName: string;
    authorAvatarUrl?: string | null;
    body: string;
    adminReply: string | null;
    createdAt: string;
}

export interface AdminCommentDto {
    id: string;
    figurineId: string;
    figurineName: string;
    authorName: string;
    authorEmail: string | null;
    body: string;
    isApproved: boolean;
    adminReply: string | null;
    createdAt: string;
    userId: string | null;
}

export interface AdminCommentsPage {
    items: AdminCommentDto[];
    total: number;
    pendingCount: number;
    page: number;
    perPage: number;
}

export interface SubmitCommentRequest {
    authorName?: string;
    authorEmail?: string;
    body: string;
}

export interface ModerateCommentRequest {
    isApproved: boolean;
    adminReply: string | null;
}

// ── Visitor impressions ("Book of Impressions") ─────────────

export interface ImpressionDto {
    id: string;
    message: string;
    authorName: string | null;
    mood: string | null;
    createdAt: string;
}

export interface AdminImpressionDto {
    id: string;
    message: string;
    authorName: string | null;
    mood: string | null;
    isApproved: boolean;
    isFeatured: boolean;
    createdAt: string;
}

export interface AdminImpressionsPage {
    items: AdminImpressionDto[];
    total: number;
    pendingCount: number;
    page: number;
    perPage: number;
}

export interface SubmitImpressionRequest {
    message: string;
    authorName?: string;
    mood?: string;
    /** Honeypot — leave empty. Any value drops the submission silently. */
    hp?: string;
}

export interface ModerateImpressionRequest {
    isApproved: boolean;
    isFeatured: boolean;
}

export interface SmtpSettings {
    host: string | null;
    port: number | null;
    user: string | null;
    pass: string | null;
    from: string | null;
}

export interface ContactSettings {
    email: string | null;
    telegram: string | null;
    phone: string | null;
}

// === CONTACT MESSAGES ("write to the author") — anonymous, two-field letters,
// not tied to a figurine (unlike OrderRequest) or a logged-in account (unlike
// MessageThreadDto). ===

export interface CreateContactMessageRequest {
    email: string;
    message: string;
    source?: string | null;
    lang?: string | null;
}

export interface ContactMessageDto {
    id: string;
    email: string;
    message: string;
    source: string;
    lang: string;
    isRead: boolean;
    createdAt: string;
}

/** Customizable "Workshop" feature block on the home page. Blank text fields
 *  fall back to the i18n defaults; blank photos fall back to the bundled ones. */
export interface ProgrammeSettings {
    maxShowings: number;
    caseBg: string | null;
    curatorNoteEn: string | null;
    curatorNoteRu: string | null;
    curatorSignEn: string | null;
    curatorSignRu: string | null;
    /** Base bronze hex for the case molding; null = built-in bronze. */
    frameTone: string | null;
    /** Molding thickness in px; null = built-in clamp. */
    frameThickness: number | null;
    /** Molding mode: 'gradient' (default) | 'flat' | 'none'; null = gradient. */
    frameMode: string | null;
}

export interface BookingRules {
    /** Minimum booking duration in days (inclusive). Default: 1 */
    minDays: number;
    /** Maximum booking duration in days (inclusive). Default: 30 */
    maxDays: number;
    /** How many days in advance booking must start (0 = today ok). Default: 0 */
    advanceDays: number;
}

export interface RescheduleBookingRequest {
    startsAt: string;
    endsAt: string;
}

export interface CreateWaitlistRequest {
    figurineName: string;
    requesterName: string;
    requesterEmail: string;
    requesterPhone: string | null;
    note: string | null;
    /** Self-certified "I am 16 or older" checkbox. */
    ageConfirmed: boolean;
}

export interface WaitlistEntryDto {
    id: string;
    figurineId: string;
    figurineName: string;
    requesterName: string;
    requesterEmail: string;
    requesterPhone: string | null;
    note: string | null;
    createdAt: string;
    userId: string | null;
    /** 1-based rank within this figurine's queue. */
    position: number;
}

export interface WaitlistCreatedResponse {
    cancelToken: string;
    position: number;
}

export interface OrderCreatedResponse {
    /** Present only for notify-mode orders. */
    cancelToken: string | null;
}

export interface NotifyInfo {
    figurineId: string;
    figurineName: string;
}

export interface WaitlistCancelInfo {
    figurineId: string;
    figurineName: string;
    position: number;
    createdAt: string;
}

// Newsletter — the house "visitor book".
export interface CreateSubscriptionRequest {
    email: string;
    name?: string | null;
    source?: string | null;
    lang?: string | null;
    /** Self-certified "I am 16 or older" checkbox. */
    ageConfirmed: boolean;
}

export interface SubscriptionCreatedResponse {
    unsubscribeToken: string;
    /** True when this email was already an active subscriber. */
    alreadySubscribed: boolean;
}

export interface SubscriberInfo {
    email: string;
}

export interface SubscriberDto {
    id: string;
    email: string;
    name: string | null;
    source: string;
    lang: string;
    createdAt: string;
}

export interface ThemeFonts {
    display: string | null;
    body: string | null;
    serif: string | null;
    mono: string | null;
}

export interface ThemeMotion {
    durationFast: string | null;
    durationDefault: string | null;
    durationSlow: string | null;
    durationGlacial: string | null;
}

export interface ThemeEffects {
    /**
     * Global "keyhole" darkness (0..1) — how deep the shadow over sealed cards
     * is. Higher hides more detail. Per-image `darkness` overrides this; null
     * falls back to the renderer default.
     */
    keyholeDarkness: number | null;
    /**
     * Seconds of dwell (hover) on a sealed card before its shadow lifts on its
     * own — no need to open the work. 0 / null disables the behaviour.
     */
    keyholeDwellReveal: number | null;
    /** Background circle color behind the raven watcher emblem. Null = default. */
    birdCircleColor: string | null;
    /** Seconds between walking-bird cameos. Null = default. */
    birdWalkInterval: number | null;
}

export interface ThemeConfig {
    colors: Record<string, string>;
    fonts: ThemeFonts;
    motion: ThemeMotion;
    effects: ThemeEffects;
}

export interface CopyOverrides {
    en: Record<string, string>;
    ru: Record<string, string>;
}

// ============================================================
// HOME LAYOUT CONFIG
// ============================================================

/** Blocks of the home page main flow. `returningBand` and `latelyShelves` are
 *  compound blocks: they move as one unit and keep their isReturningVisitor
 *  gate; their children are ordered by bandOrder / shelfOrder. */
export type HomeMainBlockId =
    | 'hero' | 'returningBand' | 'gallery' | 'authorStory' | 'correspondence'
    | 'impressions' | 'requestSteps' | 'visitorBook' | 'latelyShelves';
export type HomeBandBlockId = 'visitLedger' | 'noticeBoard';
export type HomeShelfBlockId = 'firstLook' | 'markedByYou' | 'noticedByGuests';
export type HomeBlockId = HomeMainBlockId | HomeBandBlockId | HomeShelfBlockId;

/** Width preset: full-bleed background, standard container, or narrow column. */
export type HomeBlockSize = 'full' | 'contained' | 'compact';

/** Extra vertical breathing room around a block ('base' = as designed;
 *  'tight' pulls neighbours closer with a small negative margin). */
export type HomeBlockPadding = 'tight' | 'base' | 'roomy' | 'spacious';

export type HomeDevice = 'mobile' | 'tablet' | 'desktop';

/** Per-block home overrides: DisplayConfig's BlockStyle plus layout-only knobs. */
export interface HomeBlockStyle extends BlockStyle {
    paddingY?: HomeBlockPadding;
    /** Letterpress rule drawn above the block. */
    divider?: boolean;
    /** Device classes this block is hidden on (mobile ≤680, tablet 681–1080, desktop >1080). */
    hideOn?: HomeDevice[];
}

/** Fine-grained override of ONE element inside a block (a title, a lead
 *  paragraph, the CTA row, the hero photo…). Keyed `blockId.elementId`;
 *  the element registry lives in `src/lib/home-layout.ts`. */
export interface HomeElementStyle {
    /** Text colour (cascades into the element's descendants). */
    color?: string;
    /** Free-range font size in px (text/group elements), e.g. 8–120. */
    sizePx?: number;
    /** Free-range width in % of the parent (media elements), e.g. 20–100. */
    widthPct?: number;
    hidden?: boolean;
}

/** Admin-arranged home page layout, stored as JSON in settings (`home_layout`).
 *  Absent fields mean the hard-coded default. Hidden = admin's veto; blocks
 *  that hide themselves at runtime (noticeBoard, firstLook) keep doing so. */
export interface HomeLayoutConfig {
    blockOrder?: HomeMainBlockId[];
    bandOrder?: HomeBandBlockId[];
    shelfOrder?: HomeShelfBlockId[];
    hiddenBlocks?: HomeBlockId[];
    sizes?: Partial<Record<HomeBlockId, HomeBlockSize>>;
    blockStyles?: Partial<Record<HomeBlockId, HomeBlockStyle>>;
    /** Per-element overrides, keyed `blockId.elementId`. */
    elements?: Record<string, HomeElementStyle>;
    /** Per-block order of the orderable elements inside the block's column. */
    elementOrder?: Partial<Record<HomeBlockId, string[]>>;
    /** Background of the whole home page (hex); overrides the parchment default. */
    pageBackground?: string;
    /** THE COLLECTION gallery card scroll-reveal treatment. Absent → 'rise'. */
    cardEffect?: HomeCardEffect;
}

/** THE COLLECTION gallery card scroll-reveal treatments, admin-selectable. */
export type HomeCardEffect = 'rise' | 'fog' | 'hoist' | 'drift' | 'unfold' | 'shadow';

// === REEL THEME — appearance of the home reel ===

export type BackdropKind = 'image' | 'color' | 'gradient';
export type OverlayKind = 'none' | 'solid' | 'gradient';
export type GradientType = 'linear' | 'radial' | 'conic';
export type TextTone = 'light' | 'dark';

export interface GradientStop {
    color: string;
    /** 0..100 — position along the gradient. */
    position: number;
    /** 0..1 */
    opacity: number;
}

/** Which pane a CardStyle belongs to. */
export type CardTarget = 'hero' | 'work';

/**
 * One pane's look: its glass, its type, its buttons. The opening pane and the
 * work panes carry the same fields but are set independently — they do different
 * jobs (one sells the maker, the others show the pieces).
 */
export interface CardStyle {
    // Glass
    /**
     * What the pane is filled with. 'solid' → glassTint at glassOpacity (the
     * original behaviour, and what an older saved theme with no fillKind means).
     * 'gradient' → fillStops, each stop's own alpha still scaled by glassOpacity
     * so the transparency slider keeps working in both modes.
     */
    fillKind?: CardFillKind;
    fillType?: GradientType;
    /** deg — linear only; the conic sweep also starts here. */
    fillAngle?: number;
    fillStops?: GradientStop[];
    glassTint?: string;
    glassOpacity?: number;
    /** backdrop-filter blur in px; 0 turns the frosting off. */
    glassBlur?: number;
    glassSaturation?: number;
    glassRadius?: number;
    glassSheen?: number;
    glassShadow?: number;
    shadowColor?: string;

    // Type
    titleColor?: string;
    /** rem */
    titleSize?: number;
    bodyColor?: string;
    bodySize?: number;
    /** The "01 ——" kicker on work panes; the caption line on the hero. */
    metaColor?: string;
    metaSize?: number;

    // Edge
    /**
     * The pane's hairline border, and what it becomes under the pointer. Kept as
     * plain hex (the admin uses a colour input) with the alpha as its own slider,
     * because an edge is nearly always a translucent hairline, not a solid line.
     */
    edgeColor?: string;
    edgeHoverColor?: string;
    /** 0..1 — applied to edgeColor; the hover edge gets a fixed lift above it. */
    edgeOpacity?: number;

    // Buttons
    btnFill?: string;
    btnText?: string;
    btnRadius?: number;
    btnSize?: number;
    /** Outline of the secondary (glass) button. */
    btnBorder?: string;
}

export type CardFillKind = 'solid' | 'gradient';

/**
 * Everything the admin can tune about the home reel. Every field is optional:
 * the DEFAULTS live in `$lib/stores/reel-theme.svelte.ts` and nowhere else, so
 * an empty config (a fresh install) renders exactly the designed page.
 */
export interface ReelTheme {
    // Backdrop
    backdropKind?: BackdropKind;
    backgroundImage?: string;
    /** Narrow screens get their own image — a landscape room crops to mush on a phone. */
    backgroundImageMobile?: string;
    backgroundFit?: 'cover' | 'contain';
    /** CSS object-position, e.g. "center top". */
    backgroundPosition?: string;
    backgroundBlur?: number;
    backgroundBrightness?: number;
    backgroundSaturation?: number;
    backdropColor?: string;
    /** Colour the panes' shadows are cast in — deliberately independent of the backdrop. */
    shadowColor?: string;

    // Overlay (the dimming veil)
    overlayKind?: OverlayKind;
    overlayColor?: string;
    overlayOpacity?: number;
    gradientType?: GradientType;
    gradientAngle?: number;
    gradientStops?: GradientStop[];
    vignette?: number;
    grain?: number;

    /** Drops backdrop-filter everywhere — the escape hatch for weak machines. */
    performanceMode?: boolean;

    /** Drives the rails, rules and ghost buttons. Per-card type has its own colours. */
    textTone?: TextTone;

    /** The opening pane (the house photo + manifesto), styled on its own. */
    hero?: CardStyle;
    /** Every work pane, and the closing archive pane. */
    work?: CardStyle;

    // ── Legacy flat glass/button fields ──────────────────────────────────────
    // Written by the first version of this panel, before hero and work panes were
    // separable. `resolveReelTheme` folds them into BOTH cards so an already-saved
    // theme keeps rendering as it did; nothing writes them any more.
    glassTint?: string;
    glassOpacity?: number;
    glassBlur?: number;
    glassSaturation?: number;
    glassRadius?: number;
    glassSheen?: number;
    glassShadow?: number;
    buttonFill?: string;
    buttonText?: string;

    // Density
    cardGap?: number;
    cardWidth?: number;
}

export interface ReelThemePreset {
    id: string;
    name: string;
    config: ReelTheme;
    savedAt: string;
}

export interface HomeLayoutPreset {
    id: string;
    name: string;
    config: HomeLayoutConfig;
    savedAt: string;
}

// ============================================================
// CABINET GAZETTE
// ============================================================

export type GazetteKind =
    | 'arrival'
    | 'collage'
    | 'showing'
    | 'guest_story'
    | 'tale'
    | 'note'
    | 'world'
    | 'sketch';

export type GazetteStatus = 'draft' | 'scheduled' | 'published' | 'archived';

/** Opens the gazette composer from another admin tab (a work, a showing). */
export interface GazetteSeed {
    figurineId?: string;
    kind?: GazetteKind;
    leafId?: string;
    venue?: string | null;
    startsAt?: string | null;
    endsAt?: string | null;
    imageUrls?: string[];
}

export interface GazetteLeaf {
    id: string;
    slug: string;
    kind: GazetteKind;
    status: GazetteStatus;
    titleEn: string;
    titleRu: string;
    dekEn: string | null;
    dekRu: string | null;
    bodyEn: string | null;
    bodyRu: string | null;
    figurineId: string | null;
    figurineName: string | null;
    figurineSlug: string | null;
    href: string | null;
    sourceName: string | null;
    sourceUrl: string | null;
    imageUrl: string | null;
    imageUrls?: string[];
    pinned: boolean;
    /** Place on the shelf of tall tales. Absent on every other kind of leaf. */
    shelfOrder?: number | null;
    publishedAt: string | null;
    scheduledAt: string | null;
    expectedFrom?: string | null;
    expectedTo?: string | null;
    figurineStatus?: string | null;
    watchCount?: number | null;
    createdAt: string;
    updatedAt: string;
    prev?: GazetteNeighbor | null;
    next?: GazetteNeighbor | null;
}

export interface GazetteNeighbor {
    slug: string;
    titleEn: string;
    titleRu: string;
}

export interface SaveGazetteLeafRequest {
    slug?: string | null;
    kind: GazetteKind;
    status: GazetteStatus;
    titleEn: string;
    titleRu: string;
    dekEn?: string | null;
    dekRu?: string | null;
    bodyEn?: string | null;
    bodyRu?: string | null;
    figurineId?: string | null;
    href?: string | null;
    sourceName?: string | null;
    sourceUrl?: string | null;
    imageUrl?: string | null;
    imageUrls?: string[] | null;
    pinned?: boolean;
    scheduledAt?: string | null;
    expectedFrom?: string | null;
    expectedTo?: string | null;
}

export interface GazetteLeavesPage {
    items: GazetteLeaf[];
    total: number;
    page: number;
    perPage: number;
}

export interface WatchGazetteLeafRequest {
    email?: string | null;
    name?: string | null;
    lang?: string | null;
    ageConfirmed: boolean;
}

export interface GazetteWatchCreatedResponse {
    cancelToken: string;
    alreadyWatching: boolean;
}

export interface GazetteWatchInfo {
    leafSlug: string;
    titleEn: string;
    titleRu: string;
    notified: boolean;
}

export interface GazetteWatchDto {
    id: string;
    leafId: string;
    leafSlug: string;
    titleEn: string;
    titleRu: string;
    cancelToken: string;
    notifiedAt: string | null;
    createdAt: string;
}

export interface GazetteCutting {
    id: string;
    feedId: string;
    title: string;
    url: string;
    summary: string | null;
    sourceName: string;
    publishedAt: string | null;
    dismissed: boolean;
    pinned: boolean;
    createdAt: string;
    markKey?: string;
    markUrl?: string | null;
}

export interface GazetteCuttingsPage {
    items: GazetteCutting[];
    total: number;
    page: number;
    perPage: number;
}

export interface GazetteFeed {
    id: string;
    title: string;
    url: string;
    enabled: boolean;
    lastFetchedAt: string | null;
    lastError: string | null;
    createdAt: string;
    markKey?: string;
    markUrl?: string | null;
}

export interface SaveGazetteFeedRequest {
    title: string;
    url: string;
    enabled?: boolean;
    markKey?: string | null;
    markUrl?: string | null;
}

export interface GazetteHome {
    leaves: GazetteLeaf[];
    cuttings: GazetteCutting[];
}

export interface GazetteRoom {
    year: number;
    years: number[];
    leaves: GazetteLeaf[];
    cuttings: GazetteCutting[];
}

export interface GazetteRefreshReport {
    feeds: number;
    imported: number;
    errors: string[];
}

// === СКРОМНЫЕ ЭПИЧЕСКИЕ БИТВЫ ===

export type BattleCardStatus = 'draft' | 'published' | 'retired';

/** 1..5. The card's rank — NOT the level of anyone's copy of it. */
export type BattleTier = 1 | 2 | 3 | 4 | 5;

/** One named property of a card, shown in both languages the way the keeper
 *  draws it: «Вихрь Души (Wind of Soul): …». */
export interface CardTrait {
    nameEn: string;
    nameRu: string;
    textEn: string;
    textRu: string;
}

export interface BattleCard {
    id: string;
    slug: string;
    status: BattleCardStatus;
    /** The card's rank, set by the keeper. Drives the frame and the price. */
    tier: number;
    /** The header band. */
    raceId: string | null;
    raceNameEn: string | null;
    raceNameRu: string | null;
    /** The race's shared icon, already a public URL. */
    raceIconUrl: string | null;
    /** The race's own dress per level, joined in the same way as the icon:
     *  JSON array of 5 `{frameImage?,frameMode?,aspect?}` patches, or `null`. */
    raceLevelFrames: string | null;
    typeEn: string | null;
    typeRu: string | null;
    titleEn: string;
    titleRu: string;
    /** Up to 400 characters: room for a list of named abilities on a dressed card. */
    effectEn: string | null;
    effectRu: string | null;
    loreEn: string | null;
    loreRu: string | null;
    /** Top-left corner of the card. */
    cost: number;
    /** Strength. */
    power: number;
    health: number;
    mana: number;
    traits: CardTrait[];
    // ── The body, as the engine reads it ─────────────────────────────────
    /** What this is for the rules. `typeRu` stays free text for the header band. */
    kind: BattleCardKind;
    /** Flat reduction of bodily damage. */
    armor: number;
    /** Flat reduction of charmed damage. */
    ward: number;
    /** Which defence answers this card's ordinary blow. */
    attackChannel: BattleChannel;
    /** How far the ordinary blow carries, in king's steps. 5 is the whole field. */
    reach: number;
    /** Cells walked in one move. 0 — it stands where it was put. */
    step: number;
    /** Who acts first. 3 is the middle. */
    speed: number;
    /** How much it mends in one act of mending. */
    mend: number;
    /** The executable half, beside the prose in `traits`. */
    abilities: CardAbility[];
    /** What the scales said at the last save. Desk only — the shelf never sees it. */
    budgetPoints?: number | null;
    balanceIndex?: number | null;
    /** Raised on every edit of the numbers, so a played match can still be replayed. */
    rulesVersion: number;
    /** `null` means the card is not to be had for this coin — not that it is free. */
    priceDust: number | null;
    priceFeed: number | null;
    /** Цена ступеней уровня в пыли: 1→2, 2→3, 3→4, 4→5. `null` — не растёт. */
    levelPriceDust: number[] | null;
    /** What the card wears: its own picture if it has one, else the work's face. */
    artUrl: string | null;
    /** The keeper's override as typed. Admin reads only. */
    artUrlOverride?: string | null;
    /** JSON `{x,y,zoom}` — how the picture sits in the frame. */
    artFocal: string | null;
    /** This card's own exception to the tier's shared frame: JSON
     *  `{frameImage?,frameMode?,aspect?}`. `null` wears the tier's frame as is. */
    frameOverride: string | null;
    shelfOrder?: number | null;
    figurineId: string | null;
    figurineName: string | null;
    figurineSlug: string | null;
    createdAt: string;
    updatedAt: string;
}

export interface SaveBattleCardRequest {
    slug?: string | null;
    status: BattleCardStatus;
    tier: number;
    raceId?: string | null;
    typeEn?: string | null;
    typeRu?: string | null;
    titleEn: string;
    titleRu: string;
    effectEn?: string | null;
    effectRu?: string | null;
    loreEn?: string | null;
    loreRu?: string | null;
    cost: number;
    power: number;
    health: number;
    mana: number;
    traits: CardTrait[];
    kind: BattleCardKind;
    armor: number;
    ward: number;
    attackChannel: BattleChannel;
    reach: number;
    step: number;
    speed: number;
    mend: number;
    abilities: CardAbility[];
    priceDust?: number | null;
    priceFeed?: number | null;
    /** Цена ступеней уровня в пыли: 1→2, 2→3, 3→4, 4→5. `null` — не растёт. */
    levelPriceDust?: number[] | null;
    artUrl?: string | null;
    artFocal?: string | null;
    frameOverride?: string | null;
    figurineId?: string | null;
}

export type BattleLayout = 'corners' | 'plaque';
export type BattleFrameMode = 'overlay' | 'behind' | 'sliced';

/**
 * One rank's dress. Design lives in the frame, content lives in the card.
 *
 * A frame is either PAINTED (paper/ink/border/foil, drawn by the renderer) or
 * DRESSED (`frameImage` — a photograph of a real frame laid under the card,
 * with the content set inside it by the four insets).
 */
export interface BattleFrame {
    tier: number;
    nameEn: string;
    nameRu: string;
    paper: string;
    ink: string;
    border: string;
    /** Empty string = no foil sweep at all, which is what a humble card is. */
    foil: string;
    /** A picture of a frame. Empty = painted. */
    frameImage: string;
    /**
     * `overlay` — the picture lies on top and the card shows through the hole in
     * it. What a cut-out frame with transparency wants.
     * `behind`  — the picture is the card's ground, for a frame with no hole.
     */
    frameMode: BattleFrameMode;
    /** Texture for the card's ground. A cut-out frame has nothing behind it but
     *  this. Empty = the flat `paper` colour. */
    paperImage: string;
    /** The reverse — what a card you do not own shows lying in dust. Never
     *  wears the frame itself, whatever this is set to; empty = the plain
     *  dusty texture the renderer already draws. */
    backImage: string;
    /** One corner's ornament, `frameMode: 'sliced'` only — mirrored into all
     *  four corners rather than rotated, so an asymmetric flourish stays
     *  right-side up. */
    cornerImage: string;
    /** The top edge's ornament, `sliced` only — stretched between the two
     *  corners and mirrored top-to-bottom for the foot. */
    sideImageH: string;
    /** The left edge's ornament, `sliced` only — stretched top-to-bottom and
     *  mirrored left-to-right for the other side. */
    sideImageV: string;
    /** An accent over the corner band, `sliced` only — mirrored into all four
     *  corners like `cornerImage`, but drawn in a layer above the whole
     *  9-slice assembly and shown at its own size rather than stretched. */
    cornerExtra: string;
    /** An accent centred on the top edge, `sliced` only — mirrored to the
     *  foot like `sideImageH`, laid over the assembled frame. */
    sideMidH: string;
    /** An accent centred on the left edge, `sliced` only — mirrored to the
     *  right side like `sideImageV`. */
    sideMidV: string;
    /** Where the content sits inside that photograph, in % of the card — and,
     *  in `sliced` mode, the actual width each of the four ornament bands is
     *  drawn at. */
    insetTop: number;
    insetRight: number;
    insetBottom: number;
    insetLeft: number;
    /** Card width ÷ height. */
    aspect: number;
    /**
     * The card is four bands: header, photograph, properties, footer. Three get
     * a share of the content's height; the properties band takes what is left,
     * because it is the one that has to hold a paragraph.
     * Zero is a real choice for these two — a bare card wants neither.
     */
    headerShare: number;
    /** How much of the content the work's photograph takes, 0..1. */
    artShare: number;
    footShare: number;
    /** Font id from `SITE_FONTS`. Empty = the card's ordinary serif. */
    titleFont: string;
    /** Ink for the name alone. Empty = `ink`. */
    titleInk: string;
    layout: BattleLayout;
    /** Centre of the cost badge, `corners` layout only — X in % of the card's
     *  width, Y in % of its height. */
    costX: number;
    costY: number;
    /** Centre of the power badge, same units. */
    powerX: number;
    powerY: number;
    /** The badge's own outline, independent per badge — from `BADGE_SHAPES`. */
    costShape: BattleBadgeShape;
    powerShape: BattleBadgeShape;
}

export type BattleBadgeShape = 'circle' | 'square' | 'diamond' | 'hex' | 'shield';

export interface BattleFrames {
    frames: BattleFrame[];
}

/** A race in the keeper's dictionary. Shared by many cards, so renaming it
 *  renames it everywhere at once. */
export interface BattleRace {
    id: string;
    slug: string;
    nameEn: string;
    nameRu: string;
    noteEn: string | null;
    noteRu: string | null;
    /** Shown in the header band of every card of this race. */
    iconUrl: string | null;
    /** This race's own dress per level of an owned copy — JSON array of 5
     *  `{frameImage?,frameMode?,aspect?}` patches, or `null` to wear the
     *  tier's frame at every level. */
    levelFrames: string | null;
    sortOrder?: number | null;
    /** How many cards stand under it — what a rename or a removal would touch. */
    cardCount: number;
}

export type BattleCardKind = 'unit' | 'spell' | 'relic';
export type BattleChannel = 'physical' | 'magic' | 'pure' | 'none';

/** The closed list of verbs. A new card is a new combination, never a new verb. */
export type AbilityVerb =
    | 'damage' | 'dot' | 'heal' | 'hot' | 'shield' | 'zone'
    | 'bless' | 'curse' | 'control' | 'silence' | 'disarm' | 'charm'
    | 'veil' | 'guard' | 'immune' | 'thorns' | 'move' | 'summon'
    | 'sacrifice' | 'cleanse' | 'dispel' | 'mana';

export type AbilityShape =
    | 'self' | 'one' | 'adjacent' | 'chain' | 'line' | 'radius' | 'side' | 'cell';

export type AbilityTrigger =
    | 'active' | 'onPlay' | 'onHit' | 'onDamaged' | 'onDeath' | 'turnStart' | 'aura' | 'once';

/**
 * One executable ability. Lives beside the prose in `traits`, never instead of it:
 * the prose is what gets printed, this is what the engine runs.
 */
export interface CardAbility {
    id: string;
    nameEn: string;
    nameRu: string;
    verb: AbilityVerb;
    channel: BattleChannel;
    amount: number;
    shape: AbilityShape;
    /** The number carried by `chain` (links) and `radius` (cells). */
    radius: number;
    range: number;
    /** Turns of the bearer. 0 — it happens and is over. */
    duration: number;
    trigger: AbilityTrigger;
    manaCost: number;
    cooldown: number;
    keywords: string[];
}

/**
 * What the scales say about a card still being written.
 *
 * Computed by the server on every keystroke rather than in the browser: one
 * formula, in one place. Two implementations of it would disagree by the second
 * week, and the disagreement would be found by a player.
 */
export interface BattleWeigh {
    bodyPoints: number;
    abilities: AbilityWeight[];
    totalPoints: number;
    balanceIndex: number;
    tierBudget: number;
    /** The price at which this weight would sit on the curve. */
    suggestedCost: number;
}

export interface AbilityWeight {
    id: string;
    points: number;
}

// ── Испытания и партии ───────────────────────────────────────────────────────

/** Where a card stands in a challenge's arrangement. */
export interface ChallengePlacement {
    card: string;
    x: number;
    y: number;
}

/** A challenge sets BOTH sides: it is a study, not a duel. */
export interface ChallengeSetup {
    playerBoard: ChallengePlacement[];
    playerHand: string[];
    keeperBoard: ChallengePlacement[];
    keeperHand: string[];
}

export interface BattleChallenge {
    id: string;
    slug: string;
    titleEn: string;
    titleRu: string;
    noteEn: string | null;
    noteRu: string | null;
    setup: ChallengeSetup;
    botDepth: number;
    /** Paid once per challenge, never per victory. */
    rewardDust: number;
    status: BattleCardStatus;
    sortOrder?: number | null;
    /** Whether this visitor has already been paid. Absent for a guest. */
    alreadyPaid?: boolean;
}

export interface SaveBattleChallengeRequest {
    slug?: string | null;
    titleEn: string;
    titleRu: string;
    noteEn?: string | null;
    noteRu?: string | null;
    setup: ChallengeSetup;
    botDepth: number;
    rewardDust: number;
    status: BattleCardStatus;
}

// ── Что присылает движок ─────────────────────────────────────────────────────
//
// Эти формы — не выдумка фронта, а ровно то, что сериализует `battle-core`.
// Правил здесь нет и быть не должно: клиент рисует состояние и выбирает из
// присланного списка законных действий.

export type BattleSide = 'player' | 'keeper';
export type BattleOutcome = 'player' | 'keeper' | 'draw';

/** A place on the field. Rows 0–2 are the keeper's, 3–5 the guest's. */
export interface BattleCell {
    x: number;
    y: number;
}

export interface BattleSpot {
    cell: BattleCell;
    unit: number;
}

export interface BattleBodyCard {
    /** The card's slug — look the real card up by it. Never a title: a journal
     *  outlives the session and is read in both languages. */
    name: string;
    cost: number;
    health: number;
    power: number;
    armor: number;
    ward: number;
    reach: number;
    step: number;
    mend: number;
    channel: BattleChannel;
}

export interface BattleStatus {
    name: string;
    stat: string;
    amount: number;
    turns: number;
}

/** One body on the field. Not a card: a card is a template, this is a copy. */
export interface BattleUnit {
    id: number;
    owner: BattleSide;
    reach: number;
    step: number;
    mend: number;
    channel: BattleChannel;
    /** Whether it has already acted this turn. */
    acted: boolean;
    card: BattleBodyCard;
    health: { current: number; max: number };
    power: number;
    armor: number;
    ward: number;
    shield: number;
    statuses: BattleStatus[];
    immune: BattleChannel | null;
}

export interface BattleSideState {
    hand: BattleBodyCard[];
    mana: number;
    manaMax: number;
}

export interface BattleMatchState {
    units: BattleUnit[];
    board: BattleSpot[];
    player: BattleSideState;
    keeper: BattleSideState;
    round: number;
    active: BattleSide;
    outcome: BattleOutcome | null;
    rules: { secondSideCoin: number; openingAttacks: number };
    openingAttacksUsed: number;
}

/**
 * One action, in the engine's own shape. The client never builds these by hand —
 * it picks one out of `legalActions` and sends it back unchanged.
 */
export type BattleAction =
    | { play: { handIndex: number; cell: BattleCell } }
    | { move: { unit: number; to: BattleCell } }
    | { mend: { healer: number; target: number } }
    | { attack: { attacker: number; target: number } }
    | 'endTurn';

/** Одна копия одной карты — чья-то. */
export interface BattleOwnedCard {
    cardId: string;
    level: number;
    /** Пока карту не посмотрели, она носит пометку «новая». */
    isNew: boolean;
}

/**
 * Всё, что полка должна знать о том, кто на неё смотрит.
 *
 * Одним запросом, а не тремя: полка не может нарисовать ни одной карты, пока не
 * знает, чья она, — а страница, задающая три вопроса ради одной строки, мигает.
 */
export interface BattleMe {
    dust: number;
    feed: number;
    owned: BattleOwnedCard[];
}

export interface BuyBattleCardRequest {
    cardId: string;
    currency: 'dust' | 'feed';
    /** Цена, которую видел гость. Сервер её не берёт, а сверяет. */
    expectedPrice: number;
}

export interface BuyBattleCardResponse {
    cardId: string;
    level: number;
    balance: number;
    /** false — карта уже была своей, и ничего не записано: церемония играет однажды. */
    takenNow: boolean;
}

export interface RaiseBattleCardRequest {
    cardId: string;
    /** Цена следующей ступени, которую видел владелец. Сервер сверяет. */
    expectedPrice: number;
}

export interface RaiseBattleCardResponse {
    cardId: string;
    level: number;
    balance: number;
    /** false — ступень уже была оплачена, и ничего не записано. */
    raisedNow: boolean;
}

export interface BattleAttentionResponse {
    /** Сколько осело именно сейчас. Ноль — за это уже платили. */
    dust: number;
    balance: number;
}

/** Что хранитель платит за внимание, которое дом и так считает. */
export interface BattleDustRates {
    liked: number;
    seen: number;
    read: number;
}

export type BattleEvent =
    | { played: { side: BattleSide; unit: number; cell: BattleCell; cost: number } }
    | { moved: { unit: number; from: BattleCell; to: BattleCell } }
    | {
          damaged: {
              target: number;
              /** Кто ударил. `null` у зоны, яда, шипов — урона без автора.
               *  Носится ради сцены: при дальности 4 иначе не видно, кто. */
              by: number | null;
              toHealth: number;
              toShield: number;
              channel: BattleChannel;
              source: string;
              /** Почему это число, а не то, что написано на карте. */
              trail: { step: string; from: number; to: number }[];
          };
      }
    | { immune: { target: number; by: number | null; channel: BattleChannel } }
    | { healed: { target: number; amount: number } }
    | { died: { target: number } }
    | { turnEnded: { side: BattleSide; round: number } }
    | { finished: { outcome: BattleOutcome } };

export interface BattleMatch {
    id: string;
    challengeId: string | null;
    /** The number the next action must carry. A repeat is harmless. */
    seq: number;
    state: BattleMatchState;
    legalActions: BattleAction[];
    /** What just happened, for the scene to play through. */
    events: BattleEvent[];
    outcome: BattleOutcome | null;
    /** Dust credited by this very request. Zero on every later reading. */
    rewardDust: number;
}

/**
 * The keeper's bench: an arrangement played by hand, without a trace.
 *
 * Stateless on purpose. The whole journal travels with every request and the
 * board is folded from it each time — no rows to clean up, no account, no dust,
 * and the replay property checked on every click.
 */
export interface BenchRequest {
    setup: ChallengeSetup;
    actions: BattleAction[];
    next?: BattleAction | null;
    /** Whether the far side answers by itself. */
    autoKeeper: boolean;
    /** Play the rest out with the bot on both sides. */
    playOut?: boolean;
}

export interface Bench {
    state: BattleMatchState;
    legalActions: BattleAction[];
    events: BattleEvent[];
    actions: BattleAction[];
    outcome: BattleOutcome | null;
}

/** A keyword in the dictionary. `pointValue` is where the balance rate lives. */
export interface BattleKeyword {
    id: string;
    slug: string;
    nameEn: string;
    nameRu: string;
    /** The canonical wording of the rule, one per game. */
    rulesEn: string | null;
    rulesRu: string | null;
    iconUrl: string | null;
    /** Points per unit, from the exchange table. `null` — not priced yet. */
    pointValue: number | null;
    sortOrder?: number | null;
}

export interface SaveBattleKeywordRequest {
    slug?: string | null;
    nameEn: string;
    nameRu: string;
    rulesEn?: string | null;
    rulesRu?: string | null;
    iconUrl?: string | null;
    pointValue?: number | null;
}

export interface SaveBattleRaceRequest {
    slug?: string | null;
    nameEn: string;
    nameRu: string;
    noteEn?: string | null;
    noteRu?: string | null;
    iconUrl?: string | null;
    levelFrames?: string | null;
}
