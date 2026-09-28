import { expect, pageUrl, test, waitForWsOpen } from "../fixtures";

/**
 * Playwright subset for BM-PLS2 — browser multi-tab connection sanity.
 * Full M-sweep runs via synthetic `photon-leptos-bench run --experiment bm-pls2`.
 */
test.describe("bench pls2 browser subset", () => {
  for (const tabs of [1, 4]) {
    test(`${tabs} tab(s) receive WS update`, async ({ context, request, namespace }) => {
      const url = pageUrl(namespace);
      const pages = [await context.newPage()];
      for (let i = 1; i < tabs; i += 1) {
        pages.push(await context.newPage());
      }
      for (const p of pages) {
        await p.goto(url);
        await expect(p.getByTestId("counter-value")).toHaveText("0");
      }
      for (const p of pages) {
        await waitForWsOpen(p);
      }

      await request.post("/api/counter/increment", { data: { namespace } });

      for (const p of pages) {
        await expect(p.getByTestId("counter-value")).toHaveText("1", {
          timeout: 15_000,
        });
      }
    });
  }
});
