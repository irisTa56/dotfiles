---
name: squash-merge
description: Squash-merge a pull request with a commit message drawn from the pull request's title and body for whoever reads the base branch's history rather than for the reviewer, keeping the trailers its commits carry, such as Co-Authored-By and Entire-Checkpoint, so the commit messages written along the way, which later commits can falsify, stay off the base branch.
argument-hint: "[pull request number] — omit for the current branch's"
disable-model-invocation: true
---

# Squash merge

The pull request is the argument where one is given, and otherwise the current branch's.
Read what the merge needs:

```shell
gh pr view <N> --json number,title,body,baseRefName,headRefOid,isDraft
git fetch origin pull/<N>/head <base>
```

`<base>` is the `baseRefName` it prints, and the fetch brings the head and the current base, which the commands below read.

Where the pull request is a draft, or `gh api repos/{owner}/{repo} --jq .allow_squash_merge` prints `false`, report that and stop.
Run `gh pr checks <N>`, and where a check fails or is still running, say which before asking.

## The message

The subject is the title followed by a space and `(#<N>)`, as GitHub writes it for a pull request with more than one commit.

The body is drawn from the pull request's body, since it describes the change as a whole, while each commit's message describes that commit and a later one can falsify it.
It is not a copy of that body, though: the pull request's body is written for the reviewer judging the change now, and the commit message for whoever reads the base branch's history later, so leave out what only helps a reviewer judge, such as how the change was verified or where to look in review.
A fix pushed after the pull request's body was written can falsify it too, so read it against `git diff origin/<base>...<headRefOid>` first, and where a claim no longer holds, draw the commit message from the corrected text.
Propose that correction for the pull request's body as well, shown in full apart from the commit message, since the two no longer match; on the user's yes, write it to a file of its own and run `gh pr edit <N> --body-file <that file>`, so the pull request does not keep a claim the change no longer makes.
Follow the commit message's body with a blank line and the trailers the commits carry, each once, oldest commit first, as GitHub's own squash message orders them:

```shell
git log --reverse --format='%(trailers:only,unfold)' <headRefOid> --not origin/<base> | awk 'NF && !seen[$0]++'
```

These are what a squash would otherwise lose: co-authors, and the `Entire-Checkpoint` that `entire why` reads on the base branch.

## Merging

Show the user the subject and the body in full, and merge only on their yes.
Then write the subject and the body to files in the scratchpad, since a title can hold a quote the shell would read, and run:

```shell
gh pr merge <N> --squash --subject "$(cat <subject file>)" --body-file <body file> --match-head-commit <headRefOid>
```

`--match-head-commit` refuses the merge where the head moved after the user saw the message.
Where it refuses, read the pull request again and show the new message rather than retrying.
Where the merge fails for another reason, report what `gh` says rather than working around it.
