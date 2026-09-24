import { execFileSync, spawnSync } from "node:child_process";

if (process.platform !== "darwin") {
  console.error("This build command requires macOS.");
  process.exit(1);
}

let identity = process.env.APPLE_SIGNING_IDENTITY;

if (!identity) {
  const output = execFileSync("security", ["find-identity", "-v", "-p", "codesigning"], {
    encoding: "utf8",
  });
  const identities = [...output.matchAll(/^\s*\d+\)\s+([a-f\d]{40})\s+/gim)].map(
    (match) => match[1],
  );

  if (identities.length !== 1) {
    console.error(
      `Expected exactly one valid code signing identity, found ${identities.length}. ` +
        "Set APPLE_SIGNING_IDENTITY to the identity SHA-1 shown by " +
        "`security find-identity -v -p codesigning`.",
    );
    process.exit(1);
  }

  identity = identities[0];
}

console.log(`Signing the local macOS build with ${identity}`);
const result = spawnSync(
  "npm",
  ["run", "tauri", "--", "build", "--bundles", "app", ...process.argv.slice(2)],
  {
    stdio: "inherit",
    env: { ...process.env, APPLE_SIGNING_IDENTITY: identity },
  },
);

if (result.error) {
  console.error(result.error);
}
process.exit(result.status ?? 1);
