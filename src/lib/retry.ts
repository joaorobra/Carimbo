// Retry helper for the initial store load. On a cold start the webview can mount
// and fire its first `invoke` calls before the Rust `setup()` closure has run
// `app.manage(AppState)`. Until then the backend rejects commands (the managed
// state can't be resolved), so the very first `snippets_list` / `settings_get_all`
// fails and the store would be left empty until a manual F5. Retrying with a short
// backoff bridges that gap without depending on exact startup ordering.

export interface RetryOptions {
  /** Total attempts before giving up (default 20). */
  attempts?: number;
  /** Delay between attempts in ms (default 150). */
  delayMs?: number;
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/**
 * Run `fn`, retrying on rejection up to `attempts` times with a fixed delay.
 * Resolves with the first successful result; rejects with the last error only
 * after every attempt is exhausted.
 */
export async function retry<T>(
  fn: () => Promise<T>,
  { attempts = 20, delayMs = 150 }: RetryOptions = {},
): Promise<T> {
  let lastError: unknown;
  for (let i = 0; i < attempts; i++) {
    try {
      return await fn();
    } catch (err) {
      lastError = err;
      if (i < attempts - 1) await sleep(delayMs);
    }
  }
  throw lastError;
}
