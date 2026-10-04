import type { EngineInterface, Register } from 'claude-code'
import { type OpenRecord, readRecord } from './review-loop-record'

// Supports the review-loop skill (review-loop/SKILL.md): shows the state of the
// loop's record on the current branch, and holds "The tree may not move" while
// a reviewer reads the working tree. It never writes the record.

type Repo = {
  // The working tree's root, every link resolved.
  root: string
  // Where the skill keeps this branch's record.
  record: string
}

// A spawn counts as a reviewer only while this branch's record is open, so a
// prompt that merely mentions the skill outside a loop freezes nothing.
const NAMES_REVIEW = /\braise-findings\b/
const RECORD_DIR = '/.claude/review-loop/'
const LIVE = new Set(['running', 'pending'])

// Reviewers in flight: agent id -> the root of the tree it reads. A reload
// empties it, which lifts the freeze rather than leaving a stale one.
const reviewers = new Map<string, string>()

const realPath = async ($: EngineInterface, path: string): Promise<string | undefined> =>
  (await $.fs.stat(path, { resolve: true }).catch(() => undefined))?.realPath

const locate = async ($: EngineInterface, cwd?: string): Promise<Repo | undefined> => {
  const home = await $.env.get('HOME')
  const git = await $.process
    .run(
      ['git', 'rev-parse', '--path-format=absolute', '--git-common-dir', '--show-toplevel', '--abbrev-ref', 'HEAD'],
      { timeoutMs: 5000, ...(cwd === undefined ? {} : { cwd }) },
    )
    .catch(() => undefined)
  if (home === undefined || git === undefined || git.exitCode !== 0) return undefined

  const [commonDir, top, branch] = git.stdout.trim().split('\n')
  if (!commonDir || !top || !branch) return undefined

  return {
    root: (await realPath($, top)) ?? top,
    record: `${home}/.claude/review-loop/${commonDir.replaceAll('/', '-')}/${branch}.md`,
  }
}

const openRecord = async ($: EngineInterface, repo: Repo): Promise<OpenRecord | undefined> => {
  const text = await $.fs.read(repo.record).catch(() => undefined)

  return typeof text === 'string' ? readRecord(text) : undefined
}

const showStatus = async ($: EngineInterface): Promise<void> => {
  const repo = await locate($).catch(() => undefined)
  const open = repo && (await openRecord($, repo))
  if (!repo || !open) return $.ui.status(undefined)

  const round = open.round > 0 ? `round ${open.round}` : 'no round yet'
  const isFrozen = [...reviewers.values()].includes(repo.root)
  $.ui.status(`review-loop: ${round}${isFrozen ? ' · reviewer reading, tree frozen' : ''}`)
}

// Where a path lands, an Edit's file or a Write's file not there yet: the
// nearest ancestor that exists, resolved, plus the rest as spelled.
const placed = async ($: EngineInterface, path: string): Promise<string | undefined> => {
  for (let cut = path.length; cut > 0; cut = path.lastIndexOf('/', cut - 1)) {
    const real = await realPath($, path.slice(0, cut))
    if (real !== undefined) return `${real}${path.slice(cut)}`
  }

  return undefined
}

// Drops every reviewer the session no longer runs, which covers one that was
// stopped and never completed a turn. Unsure means not frozen.
const dropEnded = async ($: EngineInterface): Promise<void> => {
  const agents = await $.agent.list().catch(() => undefined)
  const live = new Set(agents?.filter(agent => LIVE.has(agent.status)).map(agent => agent.id))
  for (const id of reviewers.keys()) if (!live.has(id)) reviewers.delete(id)
}

const frozenReason = async ($: EngineInterface, path: string, agentId?: string): Promise<string | undefined> => {
  if (reviewers.size === 0) return undefined
  await dropEnded($)
  const real = reviewers.size === 0 ? undefined : await placed($, path)
  if (real === undefined) return undefined

  for (const [id, root] of reviewers) {
    if (real !== root && !real.startsWith(`${root}/`)) continue

    return id === agentId
      ? `${$.plugin.name}: the review-loop reviewer makes no edits; return findings only.`
      : `${$.plugin.name}: a review-loop reviewer is reading this working tree, which may not move until it returns (review-loop, "Opening the round"). Wait for its findings, or stop the reviewer first; the record under ~/.claude/review-loop stays writable.`
  }

  return undefined
}

export const register: Register = on => {
  on('session.start', async ($, e, next) => {
    const started = await next(e)
    await showStatus($)

    return started
  })

  on('turn.start', async ($, e, next) => {
    await showStatus($)

    return next(e)
  })

  on('turn.complete', async ($, e, next) => {
    if (e.agentId !== undefined) reviewers.delete(e.agentId)
    await showStatus($)

    return next(e)
  })

  on('agent.spawn', async ($, e, next) => {
    const repo = !e.fork && NAMES_REVIEW.test(e.prompt) ? await locate($, e.cwd) : undefined
    const isReview = repo !== undefined && (await openRecord($, repo)) !== undefined
    const started = await next(e)
    if (!isReview || started.agentId === undefined) return started

    reviewers.set(started.agentId, repo.root)
    await showStatus($)

    return started
  })

  on('tool.call', { tool: ['Edit', 'Write'] }, async ($, e, next) => {
    const deny = await frozenReason($, e.file_path, e.agentId)
    if (deny !== undefined) return { deny }

    const ran = await next(e)
    if (e.file_path.includes(RECORD_DIR)) await showStatus($)

    return ran
  })
}
