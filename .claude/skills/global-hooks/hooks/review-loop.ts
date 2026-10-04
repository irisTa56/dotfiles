import type { EngineInterface, Register } from 'claude-code'
import { readRecord } from './review-loop-record'

// Supports the review-loop skill (review-loop/SKILL.md): shows the state of the
// loop's record on the current branch in the status line. It never writes the
// record.

const RECORD_DIR = '/.claude/review-loop/'

// Where the skill keeps the current branch's record.
const recordPath = async ($: EngineInterface): Promise<string | undefined> => {
  const home = await $.env.get('HOME')
  const git = await $.process
    .run(['git', 'rev-parse', '--path-format=absolute', '--git-common-dir', '--abbrev-ref', 'HEAD'], {
      timeoutMs: 5000,
    })
    .catch(() => undefined)
  // A failed git prints nothing, or on an unborn branch names a record that is not there.
  const [commonDir, branch] = git?.stdout.trim().split('\n') ?? []
  if (home === undefined || !commonDir || !branch) return undefined

  return `${home}${RECORD_DIR}${commonDir.replaceAll('/', '-')}/${branch}.md`
}

const statusText = async ($: EngineInterface): Promise<string | undefined> => {
  const path = await recordPath($)
  const text = path === undefined ? undefined : await $.fs.read(path).catch(() => undefined)
  const open = typeof text === 'string' ? readRecord(text) : undefined
  if (open === undefined) return undefined

  return `review-loop: ${open.round > 0 ? `round ${open.round}` : 'no round yet'}`
}

const showStatus = async ($: EngineInterface): Promise<void> =>
  $.ui.status(await statusText($).catch(() => undefined))

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
    await showStatus($)

    return next(e)
  })

  // The loop writes its record mid-turn, so reread it as the write lands.
  on('tool.call', { tool: ['Edit', 'Write'] }, async ($, e, next) => {
    const ran = await next(e)
    if (e.file_path.includes(RECORD_DIR)) await showStatus($)

    return ran
  })
}
