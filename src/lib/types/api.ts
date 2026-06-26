// src/lib/types/api.ts

export type FigurineStatus = 'available' | 'sold' | 'reserved' | 'in_progress';

export interface FigurineListItem {
    id: string;
    name: string;
    status: FigurineStatus;
    faceImageUrl: string | null;
    year?: number | null;
    sortOrder?: number;
    series?: string | null;
    technique?: string | null;
    material?: string | null;
    dimensions?: string | null;
    isFeatured?: boolean;
    createdAt?: string | null;
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
     * Optional door asset shown while the window is closed. When null the sealed
     * door is drawn procedurally (carved oak) instead.
     */
    sealedDoorImage?: string | null;
    /**
     * Optional showing room this work belongs to. When set, the room's window is
     * used instead of openFromMin/openUntilMin (mutually exclusive). null → own window.
     */
    showingRoomId?: string | null;
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

export interface ProcessStep {
    id: string;
    stepType: 'sketch' | 'prototype' | 'modeling' | 'painting' | 'finish';
    description: string | null;
    imageUrl: string;
}

export interface Figurine {
    id: string;
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
    /** Optional sealed-door asset; null → procedural carved oak door. */
    sealedDoorImage?: string | null;
    /** Showing room this work belongs to; null → uses its own window. */
    showingRoomId?: string | null;
    images: FigurineImage[];
    processSteps: ProcessStep[];
    relatedItems: FigurineListItem[];
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

export interface CabinetZone {
    id: string;
    zoneType: string;
    x: number;
    y: number;
    width: number;
    height: number;
    targetRoute: string;
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
}

export interface AuthorProfile {
    name: string;
    tagline: string | null;
    bio: string | null;
    photoUrl: string | null;
    instagram: string | null;
    telegram: string | null;
    vk: string | null;
    email: string | null;
    website: string | null;
    artstation: string | null;
    pinterest: string | null;
    youtube: string | null;
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

export interface AnalyticsFunnel {
    views: number;
    engagedViews: number;
    ctaClicks: number;
    submissions: number;
}

export interface AdminFigurineAnalyticsListItem {
    figurineId: string;
    name: string;
    status: FigurineStatus;
    faceUrl: string | null;
    signal: AnalyticsSignal;
    topSource?: string | null;
    topCountry?: string | null;
    topDevice?: string | null;
    topBrowser?: string | null;
    views: number;
    uniqueVisitors: number;
    engagedViews: number;
    ctaClicks: number;
    submissions: number;
    conversionRate: number;
}

export interface AdminFigurineAnalyticsListPage {
    items: AdminFigurineAnalyticsListItem[];
    total: number;
    summary: AnalyticsSummary;
}

export interface AdminFigurineAnalyticsDetail {
    figurine: FigurineListItem;
    signal: AnalyticsSignal;
    summary: AnalyticsSummary;
    daily: AnalyticsDailyPoint[];
    sources: AnalyticsSourcePoint[];
    countries: AnalyticsBreakdownPoint[];
    devices: AnalyticsBreakdownPoint[];
    browsers: AnalyticsBreakdownPoint[];
    referrers: AnalyticsBreakdownPoint[];
    utmSources: AnalyticsBreakdownPoint[];
    visitorCohorts: AnalyticsBreakdownPoint[];
    funnel: AnalyticsFunnel;
}

export interface AdminAnalyticsQuery {
    from?: string;
    to?: string;
    sort?: string;
    dir?: 'asc' | 'desc';
}

export interface AnalyticsEventPayload {
    eventType: 'figurine_view' | 'figurine_engaged' | 'figurine_cta_click';
    figurineId: string;
    path: string;
    referrer?: string | null;
    utmSource?: string | null;
    utmMedium?: string | null;
    utmCampaign?: string | null;
    durationMs?: number | null;
    scrollDepth?: number | null;
    ctaType?: string | null;
    pageViewId?: string | null;
    clientTs?: string;
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

export interface WorkshopFeature {
    visible: boolean;
    photoBack: string | null;
    photoFront: string | null;
    eyebrowEn: string | null;
    eyebrowRu: string | null;
    titleEn: string | null;
    titleRu: string | null;
    textEn: string | null;
    textRu: string | null;
    link1LabelEn: string | null;
    link1LabelRu: string | null;
    link1Href: string | null;
    link2LabelEn: string | null;
    link2LabelRu: string | null;
    link2Href: string | null;
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
