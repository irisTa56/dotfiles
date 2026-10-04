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

const RECORD = '/home/u/.claude/review-loop/-repo-.git/feat/x.md'

type World = {
  // The record's text; undefined for no file at the branch's path.
  record?: string
  // What `git rev-parse` exits with and prints.
  git: { exitCode: number; stdout: string }
  status: (string | undefined)[]
}

// The world beneath the mod: git, the file system and the tools.
const world = (on: On, recordText?: string): World => {
  const w: World = { record: recordText, git: { exitCode: 0, stdout: '/repo/.git\nfeat/x\n' }, status: [] }
  mock.env(on, { HOME: '/home/u' })
  on('process.run', () => ({ value: { ...w.git, stderr: '', isStdoutTruncated: false, isStderrTruncated: false } }))
  on('fs.read', (_, e) => {
    if (e.path !== RECORD || w.record === undefined) return { deny: 'ENOENT' }

    return { value: w.record }
  })
  on('ui.status', (_, e) => {
    w.status.push(e.text)

    return { value: undefined }
  })
  on('turn.start', (_, e) => ({ turnId: e.turnId }))
  on('tool.call', { tool: ['Edit', 'Write'] }, () => ({ result: {} as never }))

  return w
}

const edit = (file_path: string) =>
  ({ tool: 'Edit', tool_use_id: 'toolu_edit', file_path, old_string: 'a', new_string: 'b' }) as const

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

  test('leaves the status alone on an edit elsewhere', async ($, on) => {
    const w = world(on, record('### Round 1'))
    await $.tool.call(edit('/repo/src/a.ts'))

    expect(w.status).toEqual([])
  })

  test('shows nothing outside a git repository', async ($, on) => {
    const w = world(on, record('### Round 1'))
    w.git = { exitCode: 128, stdout: '' }
    await $.turn.start({ text: 'hi', turnId: 't1' })

    expect(w.status.at(-1)).toBeUndefined()
  })
})
