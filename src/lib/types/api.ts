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
    isFeatured?: boolean;
    createdAt?: string | null;
    thumbUrl?: string | null;
}

export interface FigurineImage {
    id: string;
    imageType: 'face' | 'detail' | 'full';
    url: string;
    originalUrl: string | null;
    thumbUrl: string | null;
    altText: string | null;
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
    ambiencePath: string | null;
    videoUrl: string | null;
    secretText: string | null;
    status: FigurineStatus;
    sortOrder: number;
    isVisible: boolean;
    isFeatured: boolean;
    series?: string | null;
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

export interface OrderRequest {
    figurineId: string;
    figurineName: string;
    requesterName: string;
    requesterEmail: string;
    requesterPhone: string | null;
    message: string | null;
    mode: 'request' | 'question' | 'notify';
}

export interface Order {
    id: string;
    figurineId: string;
    figurineName: string;
    requesterName: string;
    requesterEmail: string;
    requesterPhone: string | null;
    message: string | null;
    mode: 'request' | 'question' | 'notify';
    status: 'new' | 'seen' | 'replied';
    adminNotes: string | null;
    createdAt: string;
}

export interface OrdersPage {
    items: Order[];
    total: number;
    newCount: number;
    page: number;
    perPage: number;
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
    figurineId: string | null;
    status: CommissionStatus;
    adminNotes: string | null;
    createdAt: string;
    updatedAt: string;
    attachments: AttachmentDto[];
    threadId: string | null;
    /** Work has begun — petition can no longer be edited or deleted. */
    started: boolean;
}

export interface CommissionCreatedResponse {
    id: string;
    claimToken: string;
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
    mode: 'request' | 'question' | 'notify';
    status: 'new' | 'seen' | 'replied';
    createdAt: string;
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

export interface ThemeConfig {
    colors: Record<string, string>;
    fonts: ThemeFonts;
    motion: ThemeMotion;
}

export interface CopyOverrides {
    en: Record<string, string>;
    ru: Record<string, string>;
}
