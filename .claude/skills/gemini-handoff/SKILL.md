---
name: gemini-handoff
description: Hand a change to a Google Doc, Sheet or Slides file over to Gemini, through a request placed in the user's "Claude Outbox" Drive folder, where no connector tool in the session can make it. Use whenever a Google Workspace change runs into something the connector tools cannot do, or the user mentions Claude Outbox or a handoff to Gemini.
---

# Hand a Workspace change to Gemini

Make a change to a Google Workspace file with the connector tools wherever one can make it, as the `google-workspace` skill describes, and hand it to Gemini only where none can.
Where a change has parts each way, make what the tools can and hand over the rest; a new file is created with the connectors first, so what they cannot put in it is a change like any other.
Your part in a handoff is to write a request Gemini can act on without this conversation, to put the user one click from running it, and to check the result afterwards by reading the target.

## The rules the handoff rests on

- **The outbox is append only.** A request or attachment file is never changed or removed after it is placed.
  - To withdraw or correct one, place a new request that names the old file in its `Supersedes` line.
- **The request has to stand alone.** Gemini sees the request, its attachments and the target, nothing of this conversation, so write in the request everything that decides the change.
  - What the change draws on that is not the request's own words, such as data to enter or a source document, goes beside it as an attachment.
- **Read a Doc, Sheet or Slides file with its own product's connector**, not the Drive connector's read tool, whose Markdown rendering drops indentation, spacing and fonts.
- **Gemini edits from the target's side panel.** The user opens the target, picks the request in its Gemini side panel, and runs it there, so you hand them the target's link rather than the request's.

## Steps

1. **Find the outbox:** the folder titled `Claude Outbox` at the top of the user's My Drive.
   - Search Drive with `title = 'Claude Outbox' and mimeType = 'application/vnd.google-apps.folder' and parentId = 'root'`.
   - A folder of that name elsewhere is not the outbox.
   - If none exists there, create it in My Drive with the Drive connector's create tool and `contentMimeType: "application/vnd.google-apps.folder"`.
2. **Read the target.** Take its ID from the user's link or from a Drive search, and read it.
   - The request's Keep section is written from what you read, and step 6 compares the result against it.
3. **Name the request, and place any attachments.**
   - Name the request `YYYYMMDD-HHMM-SHORT_SLUG`, with the time from `date +%Y%m%d-%H%M` on the user's machine.
   - Place each attachment in the outbox folder (`parentId`) with the Drive connector's create tool before writing the request, so its link exists for the request to name.
     - Name it after the request, with a suffix saying what it holds.
     - Pass its content's own type as `contentMimeType`, and leave conversion on.
4. **Write and place the request** in the [format below](#request-format), in the outbox folder, under the name from step 3.
   - Pass it with `contentMimeType: "text/markdown"` and leave conversion on, so Drive turns it into a Doc.
     - A `.md` file is slow to appear among the side panel's recent files, which stalls the user at the point of picking it.
5. **Hand over.** Give the user:
   - the target file as a link;
   - the file names of the request and its attachments, to pick from the side panel;
   - one line saying they run it from the target's Gemini side panel and accept or reject its suggestions there.
6. **Check the result** when the user says the request was processed.
   - They do not report what happened, so read the target yourself and check against what you read each Keep entry, comparing with your step 2 read, and each completion criterion.
   - Report which criteria hold, which do not, and any change outside the request you notice, such as a list item that lost its marker.
   - A fix is a change like any other: make it with the connector tools where one can, and with a new request where none can.

## Request format

```markdown
# SUMMARY

- Target: TARGET_TITLE TARGET_URL
- Supersedes: SUPERSEDED_REQUEST
- Attachments: ATTACHMENTS

## Change

CHANGE

## Keep

KEEP

## Done when

- CRITERION
```

Write it in the language the user works in with Gemini.
Put text quoted from the target in a code block, so the conversion to a Doc keeps its characters rather than reading them as Markdown.
Replace each placeholder as follows.

- `SUMMARY` says what changes, in one line.
- `SUPERSEDED_REQUEST` is the file name of the request this one replaces, or `none`.
- `ATTACHMENTS` gives the title and URL of each attachment, or `none`.
- `CHANGE` is for Gemini: say what to change, pointing at places by heading or by quoting the text, and leave the means to the outcome to Gemini where the user did not fix them (fonts, spacing, wording style).
- `KEEP` says what must stay as it is, guarding what an edit tends to break in passing.
  - Name the text that must stay word for word.
  - Name the structure that must survive, such as heading levels and order, the count and kind of list items, tables, links.
- `CRITERION`, one line for each, serves both the user deciding whether to accept and you checking afterwards, so every criterion must be decidable by reading the target alone.
  - Prefer "no empty paragraphs remain" to "reads cleanly".
