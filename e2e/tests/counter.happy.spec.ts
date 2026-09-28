import { expect, pageUrl, test, waitForWsOpen } from "./fixtures";

test("publish → WS → refetch updates counter", async ({
  page,
  request,
  namespace,
}) => {
  await page.goto(pageUrl(namespace));
  await expect(page.getByTestId("counter-value")).toHaveText("0");
  await waitForWsOpen(page);

  await request.post("/api/counter/increment", { data: { namespace } });

  await expect(page.getByTestId("counter-value")).toHaveText("1", {
    timeout: 10_000,
  });
});

test("second tab receives WS update without second increment", async ({
  context,
  page,
  request,
  namespace,
}) => {
  const url = pageUrl(namespace);
  await page.goto(url);
  await expect(page.getByTestId("counter-value")).toHaveText("0");

  const page2 = await context.newPage();
  await page2.goto(url);
  await expect(page2.getByTestId("counter-value")).toHaveText("0");
  await waitForWsOpen(page);
  await waitForWsOpen(page2);

  await request.post("/api/counter/increment", { data: { namespace } });

  await expect(page.getByTestId("counter-value")).toHaveText("1", {
    timeout: 10_000,
  });
  await expect(page2.getByTestId("counter-value")).toHaveText("1", {
    timeout: 10_000,
  });
});
