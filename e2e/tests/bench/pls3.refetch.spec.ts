import { expect, pageUrl, test, waitForWsOpen } from "../fixtures";

/**
 * Playwright subset for BM-PLS3 — refetch path (WS → server fn → UI update).
 */
test("bench pls3 refetch latency smoke", async ({ page, request, namespace }) => {
  await page.goto(pageUrl(namespace));
  await expect(page.getByTestId("counter-value")).toHaveText("0");
  await waitForWsOpen(page);

  const t0 = Date.now();
  await request.post("/api/counter/increment", { data: { namespace } });
  await expect(page.getByTestId("counter-value")).toHaveText("1", {
    timeout: 15_000,
  });
  const elapsed = Date.now() - t0;
  expect(elapsed).toBeLessThan(15_000);
});
