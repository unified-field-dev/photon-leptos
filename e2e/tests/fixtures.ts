import { test as base, expect, type Page } from "@playwright/test";

export function pageUrl(namespace: string, mode?: string): string {
  const params = new URLSearchParams({ ns: namespace });
  if (mode) {
    params.set("mode", mode);
  }
  return `/?${params.toString()}`;
}

type E2eWindow = { __e2eSockets?: WebSocket[] };

/** Init script: record every WebSocket the page opens on `window.__e2eSockets`. */
export function trackWebSockets() {
  const Original = window.WebSocket;
  const sockets: WebSocket[] = [];
  (window as unknown as E2eWindow).__e2eSockets = sockets;
  window.WebSocket = class extends Original {
    constructor(...args: ConstructorParameters<typeof WebSocket>) {
      super(...args);
      sockets.push(this);
    }
  } as typeof WebSocket;
}

/**
 * Wait until the page has an open WebSocket.
 *
 * Subscriptions are live-tail only: a publish sent before the socket opens is
 * never delivered, so tests must wait here before publishing.
 */
export async function waitForWsOpen(page: Page) {
  await page.waitForFunction(
    () =>
      ((window as unknown as E2eWindow).__e2eSockets ?? []).some(
        (ws) => ws.readyState === 1,
      ),
    undefined,
    { timeout: 10_000 },
  );
}

export const test = base.extend<{ namespace: string }>({
  context: async ({ context }, use) => {
    await context.addInitScript(trackWebSockets);
    await use(context);
  },
  namespace: async ({}, use, testInfo) => {
    // Per-test isolation: fullyParallel workers must not share counters.
    await use(
      `${testInfo.project.name}-p${testInfo.parallelIndex}-${testInfo.testId}`,
    );
  },
});

export { expect };

async function resetCounter(
  request: import("@playwright/test").APIRequestContext,
  namespace: string,
) {
  await request.post("/api/counter/reset", { data: { namespace } });
}

test.beforeEach(async ({ request, namespace }) => {
  await resetCounter(request, namespace);
});

export { resetCounter };
