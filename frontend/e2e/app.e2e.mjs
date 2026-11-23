// End-to-end tests: a phone-sized browser driving the real app through the web proxy.
// Run by scripts/e2e.sh (which starts the API, the production web build and a fresh database).
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { after, before, describe, it } from "node:test";

const { chromium } = createRequire(import.meta.url)("playwright");
const URL = process.env.E2E_URL ?? "http://127.0.0.1:3100";
const API = `${URL}/api/backend/api/v1`;
const LONG = 180_000;

let browser;
let page;
const pageErrors = [];

before(async () => {
  browser = await chromium.launch();
  page = await browser.newPage({ viewport: { width: 412, height: 915 } });
  page.on("pageerror", (e) => pageErrors.push(String(e)));
  page.setDefaultTimeout(60_000);
});

after(async () => {
  await browser?.close();
});

/** A small river spot that solves in a moment (hand-solvable toy game). */
async function tinySolve() {
  const configs = await (await page.request.get(`${API}/game-configs`)).json();
  const game = (configs.find((c) => c.builtin) ?? configs[0]).config;
  const none = { bet_pct_pot: [[], [], []], raise_pct_pot: [[], [], []] };
  const spec = {
    game,
    ranges: ["AQ,JJ", "KK,43o,TT"],
    board: ["Ks", "Qd", "7h", "7c", "2s"],
    starting_pot_mbb: 10000,
    effective_stack_mbb: 10000,
    tree: {
      oop: none,
      ip: { bet_pct_pot: [[], [], [100]], raise_pct_pot: [[], [], []] },
      max_raises_per_street: 0,
      all_in_threshold_pct_stack: 100,
      add_all_in: false,
    },
    solver: {
      schema_version: 1,
      variant: { algorithm: "discounted", alpha: 1.5, beta: 0.0, gamma: 2.0 },
      averaging: "quadratic",
      stopping: {
        max_iterations: 300,
        target_exploitability_pct_pot: null,
        max_seconds: null,
        exploitability_check_interval: 100,
      },
      threads: 1,
      seed: null,
    },
  };
  const r = await page.request.post(`${API}/solver/jobs`, { data: { spec } });
  assert.equal(r.status(), 202, await r.text());
  const job = await r.json();
  for (let i = 0; i < 120; i++) {
    const j = await (await page.request.get(`${API}/solver/jobs/${job.id}`)).json();
    if (j.status === "succeeded") return j;
    assert.ok(!["failed", "cancelled"].includes(j.status), JSON.stringify(j));
    await page.waitForTimeout(500);
  }
  assert.fail("the solve did not finish");
}

describe("RiverLab end to end", () => {
  it("opens the dashboard with today's goals", async () => {
    await page.goto(URL);
    await page.getByText("day streak").waitFor();
    assert.ok(await page.getByText("Hands played").isVisible());
  });

  it("answers an Academy question through the proxy and counts it as study", async () => {
    await page.goto(`${URL}/academy/pot-odds`);
    const q = page.locator('form[aria-label="Knowledge check"]').first();
    await q.waitFor();
    await q.getByRole("textbox").first().fill("25");
    await q.getByRole("button", { name: "Check answer" }).click();
    await q.getByText(/Correct/).waitFor();
    await page.goto(URL);
    await page
      .getByText(/^1 \/ \d+$/)
      .first()
      .waitFor();
  });

  it("plays a heads-up hand at the table and replays it from the history", async () => {
    await page.goto(`${URL}/play`);
    const form = page.getByRole("form", { name: "New game" });
    await form.getByLabel("Seats (including you)").selectOption("2");
    await form.getByLabel("Bot difficulty").selectOption("medium");
    await form.getByRole("button", { name: "Sit down" }).click();
    await page.getByRole("button", { name: /Deal the first hand/ }).click({ timeout: LONG });
    const bar = page.getByLabel("Your action");
    // Check or call until the hand ends (a passive line always reaches the end).
    for (let i = 0; i < 40; i++) {
      if (await page.getByRole("button", { name: "Next hand" }).isVisible()) break;
      if (await bar.isVisible()) {
        const check = bar.getByRole("button", { name: "Check", exact: true });
        const call = bar.getByRole("button", { name: /^Call / });
        if (await check.isVisible()) await check.click();
        else if (await call.isVisible()) await call.click();
      }
      await page.waitForTimeout(300);
    }
    await page.getByRole("button", { name: "Next hand" }).waitFor();
    assert.equal(await page.getByTestId("seat").count(), 2);
    const history = page.getByLabel("Hand history");
    await history.getByRole("button").first().click();
    await page.getByLabel("Players").first().waitFor();
  });

  it("deals and grades a push/fold drill", async (t) => {
    if (!process.env.E2E_PUSH_FOLD) return t.skip("the exact push/fold table is not built here");
    await page.goto(`${URL}/trainer`);
    await page.getByRole("button", { name: "Deal" }).click({ timeout: LONG });
    await page.getByRole("button", { name: "Fold", exact: true }).click();
    await page.getByText(/No EV lost|You lost/).waitFor({ timeout: LONG });
  });

  it("solves a spot, lists it in the library and opens it", async () => {
    const job = await tinySolve();
    await page.goto(`${URL}/library`);
    const list = page.getByLabel("Saved results");
    await list.getByRole("button").first().click();
    await page.getByText("Strategy tree").waitFor();
    assert.ok(job.id);
  });

  it("deals a postflop drill from that solve and grades it by EV", async () => {
    await page.goto(`${URL}/trainer`);
    await page.getByRole("tab", { name: "Postflop" }).click();
    await page.getByRole("button", { name: "Deal" }).click();
    const spot = page.getByLabel("Current postflop drill");
    await spot.waitFor();
    await spot.getByRole("group", { name: "Your action" }).getByRole("button").first().click();
    await page.getByText(/No EV lost|You lost/).waitFor();
    await page.getByLabel("Solver view of this decision").waitFor();
  });

  it("shows analytics and flicks through a weakness when one was found", async () => {
    await page.goto(`${URL}/analytics`);
    await page.getByText("Games against the bots").waitFor();
    await page.getByText("Solver leaks").waitFor();
    await page.getByRole("button", { name: "Review with the solver" }).waitFor();
    await page
      .getByText(/Push\/fold trainer|Postflop drills/)
      .first()
      .waitFor();
    const flick = page.getByRole("button", { name: /Flick through these hands/ }).first();
    if (await flick.isVisible()) {
      await flick.click();
      await page.getByText(/1 of \d+/).waitFor();
    }
  });

  it("raised no uncaught errors in the page", () => {
    assert.deepEqual(pageErrors, []);
  });
});
