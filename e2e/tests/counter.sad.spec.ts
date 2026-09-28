import { expect, pageUrl, test, waitForWsOpen } from "./fixtures";

test("server fn error surfaces in UI", async ({ page, request, namespace }) => {
  await request.post("/api/e2e/scenario", {
    data: { namespace, fail_read: true },
  });

  await page.goto(pageUrl(namespace, "server-error"));
  await expect(page.getByTestId("counter-error")).toBeVisible({
    timeout: 10_000,
  });
  await expect(page.getByTestId("counter-value")).toHaveCount(0);
});

test("publish failure leaves counter unchanged", async ({
  page,
  request,
  namespace,
}) => {
  await request.post("/api/e2e/scenario", {
    data: { namespace, fail_publish: true },
  });

  await page.goto(pageUrl(namespace));
  await expect(page.getByTestId("counter-value")).toHaveText("0");
  await waitForWsOpen(page);

  const response = await request.post("/api/counter/increment", {
    data: { namespace },
  });
  expect(response.status()).toBe(500);

  await page.waitForTimeout(500);
  await expect(page.getByTestId("counter-value")).toHaveText("0");
});

test("WS unavailable keeps SSR baseline after increment", async ({
  page,
  request,
  namespace,
}) => {
  await page.goto(pageUrl(namespace, "no-ws"));
  await expect(page.getByTestId("counter-value")).toHaveText("0");
  await expect(page.getByTestId("ws-status")).toHaveText("disabled");

  await request.post("/api/counter/increment", { data: { namespace } });

  await page.waitForTimeout(500);
  await expect(page.getByTestId("counter-value")).toHaveText("0");
});

test("WS disconnect recovers on next publish", async ({
  page,
  request,
  namespace,
}) => {
  await page.goto(pageUrl(namespace));
  await expect(page.getByTestId("counter-value")).toHaveText("0");
  await waitForWsOpen(page);

  await page.evaluate(() => {
    for (const ws of (window as unknown as { __e2eSockets: WebSocket[] })
      .__e2eSockets) {
      ws.close();
    }
  });

  await request.post("/api/counter/increment", { data: { namespace } });
  await page.waitForTimeout(500);
  await expect(page.getByTestId("counter-value")).toHaveText("0");

  // Manual socket.close() is terminal for leptos-use; reload restores the WS subscription.
  await page.reload();
  await expect(page.getByTestId("counter-value")).toHaveText("1", {
    timeout: 10_000,
  });
  await waitForWsOpen(page);

  await request.post("/api/counter/increment", { data: { namespace } });
  await expect(page.getByTestId("counter-value")).toHaveText("2", {
    timeout: 15_000,
  });
});
