#!/usr/bin/env node
//MISE description="Check whether the upstream paths this repository pins have changed on their default branches"
//MISE tools={node="24"}
//MISE alias="shared-tasks:check"
//
// What a pin is, and what each exit code means, is in tasks/README.md under
// "upstream:pin-check".
//
// Exit codes: 0 nothing to act on, 1 a pinned path differs from its upstream's default
// branch, 2 it could not tell.
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const OK = 0;
const DIFFERS = 1;
const UNKNOWN = 2;

// Once this branch exists it is the person's who opens its pull request, who may add
// commits to it, so a later run leaves it alone; merging that pull request deletes it where
// the repository deletes merged branches, and the next run pushes afresh.
const BUMP_BRANCH = "upstream-pin-bump";
const BOT_NAME = "github-actions[bot]";
const BOT_EMAIL = "41898282+github-actions[bot]@users.noreply.github.com";

type Run = { status: number; stdout: string; stderr: string };

// One pin: an upstream URL, a path in it, and the commit pinned, with the tracked files
// that spell it.
type Pin = { url: string; path: string; ref: string; files: string[] };

// A pin that differs, and the default branch's tip it would move to.
type Move = { pin: Pin; tip: string };

// A pin as one text spelled it, and the tip the move put there in its place.
type Moved = { url: string; path: string; from: string; tip: string };

// A marker is the form mise's `git::` include takes, so a `task_config.includes` entry is
// one as it stands. The URL is a scheme, a host and the path segments after it, ending at
// the first `//` past the scheme's, which is where mise splits it too; requiring the scheme
// keeps a URL from opening with `-`, which git would take as an option. Only a full sha
// counts: a branch or a short sha moves, so there would be no fixed commit to compare
// against.
const MARKER =
  /git::([a-z][a-z0-9+.-]*:\/\/[^\s"'?/]*(?:\/[^\s"'?/]+)*?)\/\/([^\s"'?]+)\?ref=([0-9a-f]{40})(?![0-9a-f])/g;

type GitOptions = { env?: Record<string, string>; input?: string };

function git(args: string[], cwd?: string, options: GitOptions = {}): Run {
  const env = { ...process.env, ...options.env };
  const r = spawnSync("git", args, {
    cwd,
    env,
    input: options.input,
    encoding: "utf8",
  });
  return {
    status: r.status ?? UNKNOWN,
    stdout: r.stdout ?? "",
    stderr: r.stderr ?? "",
  };
}

function fail(message: string): void {
  console.error(`[fail] ${message}`);
}

// git's own reason goes out above the line that says what could not be done.
function failGit(run: Run, message: string): number {
  process.stderr.write(run.stderr);
  fail(message);
  return UNKNOWN;
}

// A git hook runs with GIT_DIR and its kin exported, which would point every git call
// here, the scratch fetches included, at the repository the hook runs for.
function clearLocalGitEnv(): void {
  const names = git(["rev-parse", "--local-env-vars"]).stdout.split("\n");
  for (const name of names) if (name) delete process.env[name];
}

function readPins(files: string[]): Pin[] {
  const pins = new Map<string, Pin>();
  for (const file of files) {
    for (const [, url, path, ref] of readFileSync(file, "utf8").matchAll(
      MARKER,
    )) {
      const key = `${url}\0${path}\0${ref}`;
      const pin = pins.get(key) ?? { url, path, ref, files: [] };
      if (!pin.files.includes(file)) pin.files.push(file);
      pins.set(key, pin);
    }
  }
  return [...pins.values()];
}

// Fetches `ref` of `url` into the scratch repository, trees without blobs, and returns the
// commit; only the trees are compared, so no file's content is downloaded.
function fetchCommit(scratch: string, url: string, ref: string): string | Run {
  const fetched = git(
    ["fetch", "-q", "--depth=1", "--filter=blob:none", url, ref],
    scratch,
  );
  if (fetched.status !== 0) return fetched;
  const commit = git(["rev-parse", "FETCH_HEAD"], scratch);
  return commit.status === 0 ? commit.stdout.trim() : commit;
}

function treeAt(
  scratch: string,
  commit: string,
  path: string,
): string | undefined {
  const tree = git(
    ["rev-parse", "-q", "--verify", `${commit}:${path}`],
    scratch,
  );
  return tree.status === 0 ? tree.stdout.trim() : undefined;
}

function marker(url: string, path: string, ref: string): string {
  return `git::${url}//${path}?ref=${ref}`;
}

// What the move brings, as a page where the host has one.
function brings(url: string, from: string, to: string): string {
  const github = url.match(
    /^(https:\/\/github\.com\/[^/]+\/[^/]+?)(?:\.git)?$/,
  );
  return github
    ? `${github[1]}/compare/${from}...${to}`
    : `${url} ${from}...${to}`;
}

function main(argv: string[]): number {
  const bump = argv[0] === "--bump";
  const pathspecs = bump ? argv.slice(1) : argv;
  const option = pathspecs.find((arg) => arg.startsWith("-"));
  if (option !== undefined) {
    fail(`unknown argument '${option}'; this takes --bump, then git pathspecs`);
    return UNKNOWN;
  }
  clearLocalGitEnv();
  // Names from the repository's root, which is where the rest of the run works, so that a
  // file reads the same from the working tree and from the default branch's tree.
  const listed = git([
    "grep",
    "-lzI",
    "--full-name",
    "-e",
    "git::",
    "--",
    ...pathspecs,
  ]);
  if (listed.status > 1)
    return failGit(
      listed,
      "could not list the tracked files to read pins from",
    );
  const root = git(["rev-parse", "--show-toplevel"]);
  if (root.status !== 0)
    return failGit(root, "could not find the repository's root");
  process.chdir(root.stdout.trim());
  const pins = readPins(listed.stdout.split("\0").filter(Boolean));
  if (pins.length === 0) {
    fail("found no upstream pin in the tracked files");
    return UNKNOWN;
  }

  const scratch = mkdtempSync(join(tmpdir(), "upstream-pin-check-"));
  try {
    const init = git(["init", "-q"], scratch);
    if (init.status !== 0)
      return failGit(
        init,
        `could not create a scratch repository at ${scratch}`,
      );
    let status = OK;
    const moves: Move[] = [];
    for (const pin of pins) {
      const result = check(scratch, pin);
      if (typeof result === "string") moves.push({ pin, tip: result });
      status = Math.max(status, typeof result === "string" ? DIFFERS : result);
    }
    if (bump && moves.length > 0) {
      const moved =
        process.env.GITHUB_ACTIONS === "true"
          ? moveOnDefaultBranch(scratch, moves)
          : moveInWorkingTree(moves);
      status = Math.max(status, moved);
    }
    return status;
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

// Returns the default branch's tip where the pinned path differs from it.
function check(scratch: string, pin: Pin): number | string {
  const where = `${pin.url}//${pin.path}`;
  const tip = fetchCommit(scratch, pin.url, "HEAD");
  if (typeof tip !== "string")
    return failGit(tip, `could not read the default branch of ${pin.url}`);
  const tipTree = treeAt(scratch, tip, pin.path);
  const pinned = fetchCommit(scratch, pin.url, pin.ref);
  if (typeof pinned !== "string")
    return failGit(pinned, `could not read ${pin.ref} from ${pin.url}`);
  const pinnedTree = treeAt(scratch, pinned, pin.path);
  if (pinnedTree === undefined) {
    fail(`${where} does not exist at the pinned ${pin.ref}`);
    return UNKNOWN;
  }
  // A pin moved to a tip without the path would fail every run after, so it stays.
  if (tipTree === undefined) {
    fail(
      `${where} is gone from the default branch (${tip}), pinned in ${pin.files.join(", ")}; a bump leaves this pin where it is`,
    );
    return DIFFERS;
  }
  if (pinnedTree === tipTree) {
    console.log(
      `[ok] ${where} at the pinned ${pin.ref} matches the default branch (${tip})`,
    );
    return OK;
  }
  // Not which is the older: the pin may be ahead of the default branch as well as behind.
  fail(
    `${where} differs from the default branch, pinned in ${pin.files.join(", ")}`,
  );
  fail(`pinned: ${pin.ref}`);
  fail(`tip:    ${tip}`);
  return tip;
}

// Moves each marker in `text` that `tipFor` gives a tip for, and returns the moved text
// with what moved, each move read from the marker it replaced.
function rewrite(
  text: string,
  tipFor: (url: string, path: string, ref: string) => string | undefined,
): { text: string; moved: Moved[] } {
  const moved: Moved[] = [];
  const result = text.replace(MARKER, (whole, url, path, from) => {
    const tip = tipFor(url, path, from);
    if (tip === undefined || tip === from) return whole;
    moved.push({ url, path, from, tip });
    return marker(url, path, tip);
  });
  return { text: result, moved };
}

// One line per distinct move, however many files or markers it touched.
function describe(moved: Moved[]): string[] {
  const lines = moved.map(
    ({ url, path, from, tip }) =>
      `${url}//${path} to ${tip}; what it brings: ${brings(url, from, tip)}`,
  );
  return [...new Set(lines)];
}

// Outside GitHub Actions the move lands in the working tree, for whoever ran it to commit.
function moveInWorkingTree(moves: Move[]): number {
  const tips = new Map(
    moves.map(({ pin, tip }) => [marker(pin.url, pin.path, pin.ref), tip]),
  );
  const files = [...new Set(moves.flatMap(({ pin }) => pin.files))];
  const moved: Moved[] = [];
  for (const file of files) {
    const result = rewrite(readFileSync(file, "utf8"), (url, path, ref) =>
      tips.get(marker(url, path, ref)),
    );
    writeFileSync(file, result.text);
    moved.push(...result.moved);
  }
  for (const line of describe(moved)) fail(`moved the pin of ${line}`);
  return DIFFERS;
}

// In GitHub Actions the checkout is whatever the run was for, often a pull request's merge
// commit, so the move is made on the repository's default branch instead and pushed to a
// branch of its own. A person opens the pull request from it: opening one with the job's
// token needs a repository setting that also lets that token approve pull requests.
function moveOnDefaultBranch(scratch: string, moves: Move[]): number {
  const exists = git([
    "ls-remote",
    "-q",
    "--exit-code",
    "origin",
    `refs/heads/${BUMP_BRANCH}`,
  ]);
  if (exists.status === 0) {
    fail(`${BUMP_BRANCH} already exists on origin, so it is left as it is`);
    return DIFFERS;
  }
  if (exists.status !== 2)
    return failGit(
      exists,
      `could not tell whether ${BUMP_BRANCH} exists on origin`,
    );

  const fetched = git(["fetch", "-q", "--depth=1", "origin", "HEAD"]);
  if (fetched.status !== 0)
    return failGit(fetched, "could not fetch origin's default branch");
  const base = git(["rev-parse", "FETCH_HEAD"]).stdout.trim();

  // Build the commit without touching the working tree or its index: a scratch index over
  // the default branch's tree, with each moved file swapped in at the mode it had there.
  // The default branch may pin another commit than the checkout, or lack a pin the
  // checkout has, so what moves, and from where, is read from its own text.
  const tips = new Map(
    moves.map(({ pin, tip }) => [`${pin.url}\0${pin.path}`, tip]),
  );
  const index = { GIT_INDEX_FILE: join(scratch, "bump-index") };
  const read = git(["read-tree", base], undefined, { env: index });
  if (read.status !== 0)
    return failGit(read, "could not read origin's default branch");
  const files = [...new Set(moves.flatMap(({ pin }) => pin.files))];
  const moved: Moved[] = [];
  for (const file of files) {
    const entry = git(["ls-tree", base, "--", file]).stdout.split(/\s/);
    const before = git(["cat-file", "blob", `${base}:${file}`]);
    if (before.status !== 0 || entry.length < 3) continue;
    const after = rewrite(before.stdout, (url, path) =>
      tips.get(`${url}\0${path}`),
    );
    if (after.moved.length === 0) continue;
    const blob = git(["hash-object", "-w", "--stdin"], undefined, {
      input: after.text,
    }).stdout.trim();
    const update = git(
      ["update-index", "--cacheinfo", `${entry[0]},${blob},${file}`],
      undefined,
      { env: index },
    );
    if (update.status !== 0)
      return failGit(update, `could not stage the move of ${file}`);
    moved.push(...after.moved);
  }
  if (moved.length === 0) {
    fail(
      "origin's default branch holds none of these pins to move, or holds their tips already",
    );
    return UNKNOWN;
  }
  const tree = git(["write-tree"], undefined, { env: index }).stdout.trim();

  const lines = describe(moved);
  const bot = {
    GIT_AUTHOR_NAME: BOT_NAME,
    GIT_AUTHOR_EMAIL: BOT_EMAIL,
    GIT_COMMITTER_NAME: BOT_NAME,
    GIT_COMMITTER_EMAIL: BOT_EMAIL,
  };
  const message = [
    "chore: move upstream pins to their default branches",
    "",
    ...lines.map((line) => `- ${line}`),
    "",
  ].join("\n");
  const commit = git(["commit-tree", tree, "-p", base, "-F", "-"], undefined, {
    env: bot,
    input: message,
  });
  if (commit.status !== 0) return failGit(commit, "could not commit the move");
  const pushed = git([
    "push",
    "-q",
    "origin",
    `${commit.stdout.trim()}:refs/heads/${BUMP_BRANCH}`,
  ]);
  if (pushed.status !== 0) {
    return failGit(
      pushed,
      `could not push the move to ${BUMP_BRANCH}; the push needs the job's token with contents: write kept on the checkout's origin`,
    );
  }
  for (const line of lines) fail(`pushed the move to ${BUMP_BRANCH}: ${line}`);
  const server = process.env.GITHUB_SERVER_URL ?? "https://github.com";
  fail(
    `open its pull request: ${server}/${process.env.GITHUB_REPOSITORY ?? ""}/compare/${BUMP_BRANCH}?expand=1`,
  );
  return DIFFERS;
}

process.exitCode = main(process.argv.slice(2));
