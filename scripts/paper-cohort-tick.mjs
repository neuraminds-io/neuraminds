#!/usr/bin/env node

import { apiPost, loginAdmin } from "./paper-cohort-lib.mjs";

async function main() {
  const { accessToken } = await loginAdmin();
  const limit = Number(process.env.PAPER_RUNNER_TICK_LIMIT || 200);
  const payload = await apiPost("/external/agents/runner/tick", accessToken, {
    limit,
  });

  console.log(JSON.stringify(payload, null, 2));
}

main().catch((error) => {
  console.error(
    JSON.stringify(
      {
        ok: false,
        message: error.message,
        status: error.status || null,
        details: error.payload || null,
      },
      null,
      2,
    ),
  );
  process.exit(1);
});
