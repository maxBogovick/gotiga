import type { TranslationKey } from '$lib/i18n';
import type { FeedItem, FeedKind } from '$lib/stores/user-desk.svelte';

/**
 * Слова для дел профиля.
 *
 * Функции берут `tr` доводом, а не читают стор сами: их зовут шесть маршрутов,
 * и подписка на язык внутри каждого вызова означала бы шесть подписок на одно.
 * В шаблоне это `$t`, то есть язык уже разрешён вызывающим.
 */
export type Tr = (key: TranslationKey) => string;

export function kindLabel(tr: Tr, kind: FeedKind): string {
  return kind === 'booking' ? tr('profileOverviewKindBooking')
    : kind === 'order' ? tr('profileOverviewKindOrder')
    : kind === 'waitlist' ? tr('profileLinkClaimKindWaitlist')
    : tr('profileOverviewKindCommission');
}

export function bookingStatusLabel(tr: Tr, status: string): string {
  const map: Record<string, TranslationKey> = {
    pending: 'profileBookingPending',
    confirmed: 'profileBookingConfirmed',
    rejected: 'profileBookingRejected',
    cancelled: 'profileBookingCancelled',
  };
  const key = map[status];
  return key ? tr(key) : status;
}

export function orderStatusLabel(tr: Tr, status: string): string {
  if (status === 'replied') return tr('profileOrderReplied');
  if (status === 'seen') return tr('profileOrderSeen');
  return tr('profileOrderNew');
}

export function orderModeLabel(tr: Tr, mode: string): string {
  const map: Record<string, TranslationKey> = {
    request: 'profileOrderModeRequest',
    question: 'profileOrderModeQuestion',
    notify: 'profileOrderModeNotify',
    reserve: 'profileOrderModeReserve',
  };
  const key = map[mode];
  return key ? tr(key) : mode;
}

export function reserveStatusLabel(tr: Tr, status: string | null): string {
  const map: Record<string, TranslationKey> = {
    requested: 'profileReserveRequested',
    reviewing: 'profileReserveReviewing',
    terms_sent: 'profileReserveTermsSent',
    confirmed: 'profileReserveConfirmed',
    declined: 'profileReserveDeclined',
    expired: 'profileReserveExpired',
  };
  const key = status ? map[status] : undefined;
  return key ? tr(key) : (status ?? tr('profileReserveRequested'));
}

/**
 * Состояние прошения. Раньше этот список был записан английскими словами прямо
 * в странице профиля и не переводился вовсе: русский посетитель читал
 * «In progress» под своим прошением.
 */
export function commissionStatusLabel(tr: Tr, status: string): string {
  const map: Record<string, TranslationKey> = {
    new: 'profileCommissionStatusNew',
    reviewing: 'profileCommissionStatusReviewing',
    accepted: 'profileCommissionStatusAccepted',
    in_progress: 'profileCommissionStatusInProgress',
    completed: 'profileCommissionStatusCompleted',
    declined: 'profileCommissionStatusDeclined',
  };
  const key = map[status];
  return key ? tr(key) : status;
}

export function wishStatusLabel(tr: Tr, status: string): string {
  const map: Record<string, TranslationKey> = {
    available: 'profileWishAvailable',
    sold: 'profileWishSold',
    reserved: 'profileWishReserved',
    in_progress: 'profileWishInProgress',
  };
  const key = map[status];
  return key ? tr(key) : status;
}

export function feedStatusLabel(tr: Tr, item: FeedItem): string {
  return item.kind === 'booking' ? bookingStatusLabel(tr, item.status)
    : item.kind === 'order' ? orderStatusLabel(tr, item.status)
    : item.kind === 'waitlist' ? `№${item.status}`
    : commissionStatusLabel(tr, item.status);
}

export function claimKindLabel(tr: Tr, kind: string): string {
  const map: Record<string, TranslationKey> = {
    booking: 'profileLinkClaimKindBooking',
    waitlist: 'profileLinkClaimKindWaitlist',
    notify: 'profileLinkClaimKindNotify',
    commission: 'profileLinkClaimKindCommission',
  };
  const key = map[kind];
  return key ? tr(key) : kind;
}

export function formatDate(lang: string, iso: string): string {
  return new Date(iso).toLocaleDateString(lang, { day: 'numeric', month: 'long', year: 'numeric' });
}

export function formatDateRange(lang: string, start: string, end: string): string {
  const opts: Intl.DateTimeFormatOptions = { day: 'numeric', month: 'short' };
  return `${new Date(start).toLocaleDateString(lang, opts)} — ${new Date(end).toLocaleDateString(lang, opts)}`;
}

/** Ключ карточки работы, к которой относится дело: по нему строится её адрес. */
export function cardKeyOf(item: FeedItem): string {
  return item.figurineId ?? `${item.kind}:${item.id}`;
}
