---
name: squash-merge
description: Squash-merge a pull request with the pull request's title and body as the commit message, keeping the trailers its commits carry, such as Co-Authored-By and Entire-Checkpoint, so the commit messages written along the way, which later commits can falsify, stay off the base branch.
argument-hint: "[pull request number] — omit for the current branch's"
disable-model-invocation: true
---

# Squash merge

The pull request is the argument where one is given, and otherwise the current branch's.
Read what the merge needs:

```shell
gh pr view <N> --json number,title,body,baseRefName,headRefOid,isDraft
```

Where the pull request is a draft, or `gh api repos/{owner}/{repo} --jq .allow_squash_merge` prints `false`, report that and stop.
Run `gh pr checks <N>`, and where a check fails or is still running, say which before asking.

## The message

The subject is the title followed by a space and `(#<N>)`, as GitHub writes it for a pull request with more than one commit.

The body is the pull request's body as it stands, since it is written to describe the change once its review has closed, while each commit's message describes that commit and a later one can falsify it.
Follow it with a blank line and the trailers the commits carry, each once, oldest commit first, as GitHub's own squash message orders them:

```shell
git fetch origin pull/<N>/head <base>
git log --reverse --format='%(trailers:only,unfold)' <headRefOid> --not origin/<base> | awk 'NF && !seen[$0]++'
```

These are what a squash would otherwise lose: co-authors, and the `Entire-Checkpoint` that `entire why` reads on the base branch.

## Merging

Show the user the subject and the body in full, and merge only on their yes.
Then write the body to a file in the scratchpad and run:

```shell
gh pr merge <N> --squash --subject '<subject>' --body-file <file> --match-head-commit <headRefOid>
```

`--match-head-commit` refuses the merge where the head moved after the user saw the message.
Where it refuses, read the pull request again and show the new message rather than retrying.
Where the merge fails for another reason, report what `gh` says rather than working around it.
