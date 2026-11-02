---
name: gemini-handoff
description: Hand a change to a Google Doc, Sheet or Slides file over to Gemini instead of making it yourself — write the request as a Doc in the user's "Claude Outbox" Drive folder, make sure the target file exists, and give the user the target's link to run the request from Gemini's side panel there; then, once the user says it was processed, read the target and check it against the request. Use whenever the user asks for a Google Workspace change to go through Gemini, mentions Claude Outbox or a handoff to Gemini, or wants a Workspace file changed where your own connector cannot edit it, as on a machine whose organization allows Claude only to create and copy files.
---

# Hand a Workspace change to Gemini

The user keeps Claude from changing Google Workspace files on its own, and an organization they work in allows Claude only to create and copy Drive files.
Gemini can still make the change, and it shows its edits as suggestions the user accepts or rejects, so the user decides what lands.
Your part is to write a request Gemini can act on without this conversation, to put the user one click from running it, and to check the result afterwards by reading the target.

## The rules the handoff rests on

- **Create and read only.** Use the Drive connector's create and read tools, and no tool that updates, moves or trashes a file, even where the session offers one; the same steps then work on a machine that allows nothing else.
- **The outbox is append only.** A request file is never changed or removed after it is placed. To withdraw or correct one, place a new request that names the old file in its `Supersedes` line.
- **The request has to stand alone.** Gemini sees the request file and the target file, nothing of this conversation, so write in the request everything that decides the change.
- **Gemini edits only from the target's side panel.** In the user's trial on 2026-11-02, neither the Gemini app nor Gemini in Drive could edit the target file, and the side panel of the open target could, with the request file picked from it. That is why you hand the user the target's link rather than the request's.

## Steps

1. **Find the outbox.** Search Drive for a folder titled `Claude Outbox` (`title = 'Claude Outbox' and mimeType = 'application/vnd.google-apps.folder'`; Drive search takes `title`, not `name`). If none exists, create it in My Drive.
2. **Make sure the target exists.** For a change to an existing file, take its ID from the user's link or from a Drive search. For a new file, create an empty one of the right type, with no content:
   - Doc: `application/vnd.google-apps.document`
   - Sheet: `application/vnd.google-apps.spreadsheet`
   - Slides: `application/vnd.google-apps.presentation`
3. **Write the request** in the [format below](#request-format).
4. **Place the request** with the Drive connector's create tool in the outbox folder (`parentId`), and `contentMimeType: "text/markdown"`, leaving conversion on so Drive turns it into a Doc: a `.md` file did not show among the side panel's recent files right away, which stalls the user at the point of picking it. Name it `YYYYMMDD-HHMM-<short-slug>`, taking the time from `date +%Y%m%d-%H%M` on the user's machine.
5. **Read it back** with the Drive connector's read tool and compare it with what you wrote.
6. **Hand over.** Give the user the target file as a link, the request's file name to pick from the side panel, and one line saying they run it from the target's Gemini side panel and accept or reject its suggestions there.
7. **Check the result** when the user says the request was processed. They do not report what happened, so read the target yourself and check each completion criterion against what you read. Report which criteria hold, which do not, and any change outside the request you notice, such as a list item that lost its marker. Leave the fix to the user or to a new request; you do not edit the target.

## Request format

```markdown
# <What changes, in one line>

- Target: <target file title> <target URL>
- Supersedes: <file name of the request this replaces, or "none">

## Change

<What to change and how. Point at places by heading or by quoting the text.>

## Keep

<What must stay as it is.>

## Done when

- <A criterion checkable by reading the target>
```

Write it in the language the user works in with Gemini.

- **Change** is for Gemini: say the outcome and leave the means to it where the user did not fix them (fonts, spacing, wording style).
- **Keep** guards what an edit tends to break in passing. Name the text that must stay word for word and the structure that must survive: heading levels and order, the count and kind of list items, tables, links. In the first trial Gemini cleaned up spacing as asked and also turned the last item of a numbered list into a plain paragraph.
- **Done when** serves both the user deciding whether to accept and you checking afterwards, so every criterion must be decidable by reading the target alone. Prefer "no empty paragraphs remain" to "reads cleanly".
