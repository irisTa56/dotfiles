// Reads a review-loop record (review-loop/SKILL.md, "The record"). A model keeps
// the file as prose, so this parses loosely and answers undefined, which shows
// no status, wherever it cannot tell.

export type OpenRecord = {
  // The highest round a verdict group names; 0 before the first one.
  round: number
}

const CLOSED_HEADING = /^##\s+Closed\b/
// Records also end on a bare sentence in place of the heading.
const CLOSED_SENTENCE = /^(Closed|Withdrawn)\b/
const VERDICTS = /^##\s+Verdicts\b/
// "### Round 3 — held", "## Round 4 (endgame)", "### Rounds 1-6 — ...", "**Round 2.**"
const ROUND = /^(?:#{2,4}\s+|\*\*)Rounds?\s+(\d+)(?:\s*[-–]\s*(\d+))?/

const isClosed = (lines: readonly string[]): boolean => {
  const last = lines.at(-1) ?? ''
  if (CLOSED_HEADING.test(last) || CLOSED_SENTENCE.test(last)) return true
  // The closing commit, or a note made after the close, often follows the
  // heading; only a round group after it is a reopened loop.
  const closedAt = lines.findLastIndex(line => CLOSED_HEADING.test(line))

  return closedAt >= 0 && !lines.slice(closedAt + 1).some(line => ROUND.test(line))
}

export const readRecord = (text: string): OpenRecord | undefined => {
  const lines = text.split('\n').filter(line => line.trim() !== '')
  const verdictsAt = lines.findIndex(line => VERDICTS.test(line))
  if (verdictsAt < 0 || isClosed(lines)) return undefined

  const rounds = lines.slice(verdictsAt + 1).flatMap(line => {
    const found = ROUND.exec(line)

    return found ? [Number(found[2] ?? found[1])] : []
  })

  return { round: Math.max(0, ...rounds) }
}
