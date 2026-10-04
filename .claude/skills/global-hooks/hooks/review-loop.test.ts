import type { On } from 'claude-code'
import { describe, expect, mock, test } from 'claude-code/testing'
import { readRecord } from './review-loop-record'

const HEAD = ['# Review-loop record', '', '## Background (sent to the reviewer)', '', 'The purpose.', '']
const record = (...verdicts: string[]): string => [...HEAD, '## Verdicts (never sent)', '', ...verdicts].join('\n')

describe('readRecord', () => {
  test('counts no round before the first verdict group', () => {
    expect(readRecord(record('Title: fix(x): y', 'Baseline: abc1234'))).toEqual({ round: 0 })
  })

  test('reads the round from headings of either depth', () => {
    expect(readRecord(record('### Round 1 — 3 findings', 'text', '## Round 2 (endgame)', 'text'))).toEqual({ round: 2 })
  })

  test('takes the highest round, whatever order the groups are in', () => {
    expect(readRecord(record('### Rounds 1-6 — the shape', '### Round 9', '### Round 8', '**Round 7.** text'))).toEqual({
      round: 9,
    })
  })

  test('ignores a round named in prose or in the background', () => {
    const text = [...HEAD, '### Where it sits (round 4)', '## Verdicts', 'Round 5 applied 3 fixes.'].join('\n')
    expect(readRecord(text)).toEqual({ round: 0 })
  })

  test('answers nothing for a closed record', () => {
    expect(readRecord(record('### Round 1', '## Closed abc1234', ''))).toBeUndefined()
    expect(readRecord(record('### Round 1', '## Closed', '', '`abc1234` — the commit the branch stood at'))).toBeUndefined()
    expect(readRecord(record('### Round 1', 'Closed at `abc1234`, the commit the branch stood at.'))).toBeUndefined()
    expect(readRecord(record('### Round 1', '## Closed abc1234', '## Post-close note', 'Applied without a round.'))).toBeUndefined()
  })

  test('reads a record reopened under its closing line as open', () => {
    expect(readRecord(record('### Round 1', '## Closed abc1234', '### Round 2 (reopened)'))).toEqual({ round: 2 })
  })

  test('answers nothing for a file with no verdict section', () => {
    expect(readRecord('# Working notes\n\n### Round 3\n')).toBeUndefined()
    expect(readRecord('')).toBeUndefined()
  })
})

const ROOT = '/repo'
const RECORD = '/home/u/.claude/review-loop/-repo-.git/feat/x.md'
const SPAWN = {
  tool_use_id: 'toolu_spawn',
  description: 'Review the change',
  subagentType: 'general-purpose',
  provider: { plugin: 'engine', tier: 'core' },
  parentModel: 'claude-opus-5-5',
  background: true,
  fork: false,
} as const
const REVIEW = { ...SPAWN, prompt: 'Review the current changes by running the raise-findings skill.' }

type World = {
  // The record's text; undefined for no file at the branch's path.
  record?: string
  agents: { id: string; status: string }[]
  status: (string | undefined)[]
  edits: string[]
}

// The world beneath the mod: git, the file system, the agent list and the tools.
const world = (on: On, recordText?: string): World => {
  const w: World = { record: recordText, agents: [], status: [], edits: [] }
  mock.env(on, { HOME: '/home/u' })
  on('process.run', () => ({
    value: {
      exitCode: 0,
      stdout: `${ROOT}/.git\n${ROOT}\nfeat/x\n`,
      stderr: '',
      isStdoutTruncated: false,
      isStderrTruncated: false,
    },
  }))
  on('fs.read', (_, e) => {
    if (e.path !== RECORD || w.record === undefined) return { deny: 'ENOENT' }

    return { value: w.record }
  })
  on('fs.stat', (_, e) => {
    // Only the repository's root and the record's folder exist.
    if (e.path !== ROOT && e.path !== '/home/u/.claude/review-loop') return { deny: 'ENOENT' }

    return { value: { kind: 'dir', size: 0, mtimeMs: 0, isLink: false, realPath: e.path } }
  })
  on('agent.list', () => ({
    value: w.agents.map(agent => ({ ...agent, description: 'Review the change', type: 'general-purpose' })),
  }))
  on('agent.spawn', () => {
    const agentId = `agent-${w.agents.length + 1}`
    w.agents.push({ id: agentId, status: 'running' })

    return { model: 'claude-opus-5-5', agentId }
  })
  on('ui.status', (_, e) => {
    w.status.push(e.text)

    return { value: undefined }
  })
  on('turn.start', (_, e) => ({ turnId: e.turnId }))
  on('turn.complete', (_, e) => ({ text: e.answer }))
  on('tool.call', { tool: ['Edit', 'Write'] }, (_, e) => {
    w.edits.push(e.file_path)

    return { result: {} as never }
  })

  return w
}

const edit = (file_path: string, agentId?: string) => ({
  tool: 'Edit' as const,
  tool_use_id: 'toolu_edit',
  file_path,
  old_string: 'a',
  new_string: 'b',
  ...(agentId === undefined ? {} : { agentId }),
})
const reviewerDone = (agentId: string) =>
  ({ answer: 'findings', durationMs: 1, isAborted: false, turnId: 't1', agentId, reason: 'answer' }) as const

describe('the tree freeze', () => {
  test('denies an edit inside the repository while the reviewer is in flight', async ($, on) => {
    const w = world(on, record('### Round 1'))
    await $.agent.spawn(REVIEW)

    const main = await $.tool.call(edit(`${ROOT}/src/a.ts`))
    expect(main.deny).toMatch(/may not move/)
    const written = await $.tool.call({ tool: 'Write', tool_use_id: 'toolu_write', file_path: `${ROOT}/new/dir/b.ts`, content: '' })
    expect(written.deny).toMatch(/may not move/)
    const reviewer = await $.tool.call(edit(`${ROOT}/src/a.ts`, 'agent-1'))
    expect(reviewer.deny).toMatch(/makes no edits/)
    expect(w.edits).toEqual([])
    expect(w.status.at(-1)).toBe('review-loop: round 1 · reviewer reading, tree frozen')
  })

  test('keeps the record and anything else outside the repository writable', async ($, on) => {
    const w = world(on, record('### Round 1'))
    await $.agent.spawn(REVIEW)

    expect((await $.tool.call(edit(RECORD))).deny).toBeUndefined()
    expect((await $.tool.call(edit('/repo-other/a.ts'))).deny).toBeUndefined()
    expect(w.edits).toEqual([RECORD, '/repo-other/a.ts'])
  })

  test('lifts when the reviewer completes its turn', async ($, on) => {
    const w = world(on, record('### Round 1'))
    await $.agent.spawn(REVIEW)
    await $.turn.complete(reviewerDone('agent-1'))

    expect((await $.tool.call(edit(`${ROOT}/src/a.ts`))).deny).toBeUndefined()
    expect(w.status.at(-1)).toBe('review-loop: round 1')
  })

  test('lifts when the reviewer was stopped without completing', async ($, on) => {
    const w = world(on, record('### Round 1'))
    await $.agent.spawn(REVIEW)
    w.agents[0]!.status = 'killed'

    expect((await $.tool.call(edit(`${ROOT}/src/a.ts`))).deny).toBeUndefined()
  })

  test('holds while a second reviewer still reads after the first returned', async ($, on) => {
    world(on, record('### Round 1'))
    await $.agent.spawn(REVIEW)
    await $.agent.spawn(REVIEW)
    await $.turn.complete(reviewerDone('agent-1'))

    expect((await $.tool.call(edit(`${ROOT}/src/a.ts`))).deny).toMatch(/may not move/)
  })

  test('does not freeze on a spawn that is no reviewer', async ($, on) => {
    world(on, record('### Round 1'))
    await $.agent.spawn({ ...SPAWN, prompt: 'Find where the parser lives.' })
    await $.agent.spawn({ ...REVIEW, fork: true })

    expect((await $.tool.call(edit(`${ROOT}/src/a.ts`))).deny).toBeUndefined()
  })

  test('does not freeze where no loop is open on the branch', async ($, on) => {
    const w = world(on)
    await $.agent.spawn(REVIEW)
    expect((await $.tool.call(edit(`${ROOT}/src/a.ts`))).deny).toBeUndefined()

    w.record = record('### Round 1', '## Closed abc1234')
    await $.agent.spawn(REVIEW)
    expect((await $.tool.call(edit(`${ROOT}/src/a.ts`))).deny).toBeUndefined()
  })
})

describe('the status line', () => {
  test('shows the round of an open record and nothing otherwise', async ($, on) => {
    const w = world(on)
    await $.turn.start({ text: 'hi', turnId: 't1' })
    expect(w.status.at(-1)).toBeUndefined()

    w.record = record('Title: fix(x): y')
    await $.turn.start({ text: 'hi', turnId: 't2' })
    expect(w.status.at(-1)).toBe('review-loop: no round yet')

    w.record = record('### Round 1', '### Round 2 — held')
    await $.turn.start({ text: 'hi', turnId: 't3' })
    expect(w.status.at(-1)).toBe('review-loop: round 2')

    w.record = record('### Round 1', '### Round 2', '## Closed abc1234')
    await $.turn.start({ text: 'hi', turnId: 't4' })
    expect(w.status.at(-1)).toBeUndefined()
  })

  test('rereads the record once the loop writes it', async ($, on) => {
    const w = world(on, record('### Round 1'))
    w.record = record('### Round 1', '### Round 2')
    await $.tool.call(edit(RECORD))

    expect(w.status.at(-1)).toBe('review-loop: round 2')
  })
})
