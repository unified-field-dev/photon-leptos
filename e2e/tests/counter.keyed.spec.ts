import { expect, test, trackWebSockets, waitForWsOpen } from "./fixtures";

const baseURL = process.env.LEPTOS_SITE_ADDR
  ? `http://${process.env.LEPTOS_SITE_ADDR}`
  : "http://127.0.0.1:3000";

type KeyedCookies = {
  user?: string;
  key?: string;
};

async function openKeyedPage(
  browser: import("@playwright/test").Browser,
  namespace: string,
  path: string,
  extra: KeyedCookies,
) {
  const context = await browser.newContext();
  await context.addInitScript(trackWebSockets);
  const cookies: Parameters<typeof context.addCookies>[0] = [
    { name: "e2e_ns", value: namespace, url: baseURL },
  ];
  if (extra.user) {
    cookies.push({ name: "e2e_user", value: extra.user, url: baseURL });
  }
  if (extra.key) {
    cookies.push({ name: "e2e_key", value: extra.key, url: baseURL });
  }
  await context.addCookies(cookies);

  const params = new URLSearchParams({ ns: namespace });
  if (extra.user) params.set("user", extra.user);
  if (extra.key) params.set("key", extra.key);

  const page = await context.newPage();
  await page.goto(`${path}?${params.toString()}`);
  return { context, page };
}

async function openKeyedPair(
  browser: import("@playwright/test").Browser,
  namespace: string,
  path: string,
  a: KeyedCookies,
  b: KeyedCookies,
) {
  const left = await openKeyedPage(browser, namespace, path, a);
  const right = await openKeyedPage(browser, namespace, path, b);
  await expect(left.page.getByTestId("counter-value")).toHaveText("0");
  await expect(right.page.getByTestId("counter-value")).toHaveText("0");
  await waitForWsOpen(left.page);
  await waitForWsOpen(right.page);
  return {
    pageA: left.page,
    pageB: right.page,
    contexts: [left.context, right.context],
  };
}

async function publishKeyed(
  request: import("@playwright/test").APIRequestContext,
  namespace: string,
  partition: string,
) {
  const res = await request.post("/api/counter/increment-keyed", {
    data: { namespace, partition },
  });
  expect(res.status()).toBe(204);
}

test("auth-only isolation both directions", async ({
  browser,
  request,
  namespace,
}) => {
  const { pageA, pageB, contexts } = await openKeyedPair(
    browser,
    namespace,
    "/auth-only",
    { user: "1234" },
    { user: "1235" },
  );

  try {
    await publishKeyed(request, namespace, "1234");
    await expect(pageA.getByTestId("counter-value")).toHaveText("1", {
      timeout: 10_000,
    });
    await expect(pageB.getByTestId("counter-value")).toHaveText("0");

    await publishKeyed(request, namespace, "1235");
    await expect(pageB.getByTestId("counter-value")).toHaveText("1", {
      timeout: 10_000,
    });
    await expect(pageA.getByTestId("counter-value")).toHaveText("1");
  } finally {
    await Promise.all(contexts.map((c) => c.close()));
  }
});

test("key-only isolation both directions", async ({
  browser,
  request,
  namespace,
}) => {
  const { pageA, pageB, contexts } = await openKeyedPair(
    browser,
    namespace,
    "/key-only",
    { key: "1234" },
    { key: "1235" },
  );

  try {
    await publishKeyed(request, namespace, "1234");
    await expect(pageA.getByTestId("counter-value")).toHaveText("1", {
      timeout: 10_000,
    });
    await expect(pageB.getByTestId("counter-value")).toHaveText("0");

    await publishKeyed(request, namespace, "1235");
    await expect(pageB.getByTestId("counter-value")).toHaveText("1", {
      timeout: 10_000,
    });
    await expect(pageA.getByTestId("counter-value")).toHaveText("1");
  } finally {
    await Promise.all(contexts.map((c) => c.close()));
  }
});

test("auth+key combination isolation both directions", async ({
  browser,
  request,
  namespace,
}) => {
  const { pageA, pageB, contexts } = await openKeyedPair(
    browser,
    namespace,
    "/auth-key",
    { user: "1234", key: "1234" },
    { user: "1235", key: "1235" },
  );

  try {
    await publishKeyed(request, namespace, "1234");
    await expect(pageA.getByTestId("counter-value")).toHaveText("1", {
      timeout: 10_000,
    });
    await expect(pageB.getByTestId("counter-value")).toHaveText("0");

    await publishKeyed(request, namespace, "1235");
    await expect(pageB.getByTestId("counter-value")).toHaveText("1", {
      timeout: 10_000,
    });
    await expect(pageA.getByTestId("counter-value")).toHaveText("1");
  } finally {
    await Promise.all(contexts.map((c) => c.close()));
  }
});

test("auth+key mismatch rejected while peer still receives", async ({
  browser,
  request,
  namespace,
}) => {
  const good = await openKeyedPage(browser, namespace, "/auth-key", {
    user: "1234",
    key: "1234",
  });
  const bad = await openKeyedPage(browser, namespace, "/auth-key", {
    user: "1234",
    key: "1235",
  });

  try {
    await expect(good.page.getByTestId("counter-value")).toHaveText("0");
    // Mismatch upgrade fails; UI may stay loading or show SSR baseline 0 without live sync.
    await expect(
      bad.page
        .getByTestId("counter-value")
        .or(bad.page.getByTestId("counter-loading")),
    ).toBeVisible({ timeout: 10_000 });
    await waitForWsOpen(good.page);

    await publishKeyed(request, namespace, "1234");
    await expect(good.page.getByTestId("counter-value")).toHaveText("1", {
      timeout: 10_000,
    });

    await bad.page.waitForTimeout(500);
    const badText = await bad.page
      .getByTestId("counter-value")
      .textContent()
      .catch(() => null);
    expect(badText === null || badText === "0").toBeTruthy();
  } finally {
    await Promise.all([good.context.close(), bad.context.close()]);
  }
});

test("auth=user without identity rejected while peer receives", async ({
  browser,
  request,
  namespace,
}) => {
  const good = await openKeyedPage(browser, namespace, "/auth-only", {
    user: "1234",
  });
  const bad = await openKeyedPage(browser, namespace, "/auth-only", {});

  try {
    await expect(good.page.getByTestId("counter-value")).toHaveText("0");
    await waitForWsOpen(good.page);

    await publishKeyed(request, namespace, "1234");
    await expect(good.page.getByTestId("counter-value")).toHaveText("1", {
      timeout: 10_000,
    });

    await bad.page.waitForTimeout(500);
    const badHasValue = await bad.page.getByTestId("counter-value").count();
    if (badHasValue > 0) {
      await expect(bad.page.getByTestId("counter-value")).toHaveText("0");
    }
  } finally {
    await Promise.all([good.context.close(), bad.context.close()]);
  }
});
