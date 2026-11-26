import { expect, test, type Page } from "@playwright/test";

const editor = (page: Page) => page.getByRole("textbox", { name: "Procflow source" });

async function setSource(page: Page, text: string) {
  await editor(page).fill(text);
}

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("No problems found.")).toBeVisible(); // the engine has loaded and analyzed the default example
});

test("loads the Rust engine, analyzes the default example and draws its diagram", async ({ page }) => {
  await expect(page.getByRole("img", { name: /Diagram of process Order: 6 steps and 6 connections/ })).toBeVisible();
  await expect(page.locator("svg [data-node]")).toHaveCount(6);
  await expect(page.locator("svg [data-edge]")).toHaveCount(6);
});

for (const width of [1280, 390]) {
  test(`the default diagram fits its panel at ${width}px wide`, async ({ page }) => {
    await page.setViewportSize({ width, height: 800 });
    const box = page.locator("figure.diagram .scroll");
    await expect(box).toBeVisible();
    expect(await box.evaluate((el) => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(0);
  });
}

test("a typo is underlined, listed with a suggestion, and clicking it selects exactly that text", async ({ page }) => {
  await setSource(page, "process P {\n  start -> a\n  task a -> sihp\n  task ship -> done\n  end done\n}");
  const item = page.getByRole("button", { name: /E102/ });
  await expect(item).toContainText("did you mean `ship`?");
  await expect(page.locator(".highlight .sq-error")).toHaveText("sihp");
  await item.click();
  const picked = await editor(page).evaluate((el: HTMLTextAreaElement) => el.value.slice(el.selectionStart, el.selectionEnd));
  expect(picked).toBe("sihp");
  await expect(editor(page)).toBeFocused();
});

test("positions stay right after emoji and accents", async ({ page }) => {
  await setSource(page, "// 📦 café 🎉\nprocess P {\n  start -> nope\n  end e\n}");
  await page.getByRole("button", { name: /E102/ }).click();
  const picked = await editor(page).evaluate((el: HTMLTextAreaElement) => el.value.slice(el.selectionStart, el.selectionEnd));
  expect(picked).toBe("nope");
  await expect(page.locator(".highlight .sq-error")).toHaveText("nope");
});

test("format tidies valid text, and explains itself when the text has syntax errors", async ({ page }) => {
  await setSource(page, "process P{start->a task a->e end e}");
  await page.getByRole("button", { name: "Format" }).click();
  await expect(editor(page)).toHaveValue("process P {\n  start -> a\n  task a -> e\n  end e\n}\n");
  await setSource(page, "process P { start a }");
  await page.getByRole("button", { name: "Format" }).click();
  await expect(page.getByText(/Fix the syntax errors first/)).toBeVisible();
  await expect(editor(page)).toHaveValue("process P { start a }"); // unchanged
});

test("the mistakes tour shows errors and warnings and a diagram per process", async ({ page }) => {
  await page.getByLabel("Example").selectOption({ label: "A tour of the mistakes it catches" });
  const list = page.getByRole("list", { name: "Problems" });
  for (const code of ["E101", "E102", "E106", "E107", "E108", "W201", "E012", "E014", "E001"]) {
    await expect(list.getByText(code, { exact: true }).first()).toBeVisible();
  }
  await expect(page.locator("figure.diagram")).toHaveCount(4);
  await expect(page.locator(".node.unreachable").first()).toBeVisible();
});

test("an incomplete document shows a message instead of an empty diagram", async ({ page }) => {
  await setSource(page, "// only a comment");
  await expect(page.getByText(/Nothing to draw yet/)).toBeVisible();
  await expect(page.getByText("No problems found.")).toBeVisible();
});

test("diagnostics are keyboard-reachable buttons", async ({ page }) => {
  await setSource(page, "process P { start -> x\n end e }");
  const first = page.getByRole("list", { name: "Problems" }).getByRole("button").first();
  await first.focus();
  await page.keyboard.press("Enter");
  await expect(editor(page)).toBeFocused();
});

// 343 is a 360 px phone once a classic (always visible) scrollbar takes 17 px: the case that CI's browser
// showed and a desktop browser with overlay scrollbars hides.
for (const width of [320, 343, 360, 390]) {
  test(`no sideways page scroll at ${width}px wide`, async ({ page }) => {
    await page.setViewportSize({ width, height: 740 });
    await page.goto("/");
    await expect(page.getByText("No problems found.")).toBeVisible();
    await page.getByLabel("Example").selectOption({ label: "A tour of the mistakes it catches" }); // many problems, wide text
    await expect(page.getByRole("list", { name: "Problems" })).toBeVisible();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    expect(overflow).toBeLessThanOrEqual(0);
  });
}
