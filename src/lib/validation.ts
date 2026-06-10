// Shared input validation helpers.

// Pragmatic email check: one @, a dot-separated domain, no whitespace.
// Not RFC-perfect (that's impossible with a regex) but rejects the obvious junk
// that `value.includes('@')` lets through.
const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function isValidEmail(value: string): boolean {
  return EMAIL_RE.test(value.trim());
}
