---
name: organize-reading-note
description: Organize reading notes into structured bullet points. Use when the user provides a reference (URL, PDF, etc.) with raw notes containing highlights and excerpts they found interesting. If a PDF is provided, automatically extracts highlights and comments from the PDF. The skill organizes these notes into coherent bullet points while preserving the original content, selects appropriate tags, and follows the reference's language.
---

# Reading Notes Organizer

Organize raw reading notes into structured, coherent bullet points.

## Workflow

1. **Receive input**: User provides a reference (URL, PDF, etc.) and optionally raw notes (highlights, excerpts, impressions)
2. **Extract PDF highlights** (if PDF provided):
   - Use Read tool to read the PDF
   - Visually identify highlighted text (yellow/colored backgrounds)
   - Extract highlighted text and any associated comments/annotations
   - Combine with user's raw notes if provided
3. **Fetch reference**: If URL provided, use WebFetch to understand the source content
4. **Summarize**: Write 1-3 bullet points capturing the reference's main thesis (especially if user's notes miss key points)
5. **Analyze notes**: Identify the main themes and logical groupings in the extracted/provided notes
6. **Select tags**: Review `tags/` directory and select 2-5 appropriate tags; propose new tags if needed
7. **Organize notes**: Structure notes into hierarchical bullet points
8. **Output**: Replace the raw notes in the user's file with organized sections using Edit tool

## Output Format

The frontmatter, title, and reference URL are pre-filled by the user.
Replace the raw notes with `## Tags`, `## Summary`, and `## Notes` sections:

```markdown
## Tags

- [[:tag-name]]
- [[:another-tag]]

## Summary

- [Main thesis or key insight 1]
- [Key insight 2 if applicable]
- [Key insight 3 if applicable]

## Notes

- Main point 1
  - Supporting detail
  - Another detail
- Main point 2
  - ...
```

## Guidelines

### PDF Highlight Extraction

When a PDF is provided:

- Read the PDF using Read tool (all pages or specific ranges as needed)
- Visually identify highlighted text (yellow, green, or other colored backgrounds)
- Extract the highlighted text carefully, preserving exact wording
- Look for any associated comments or annotations near highlights
- If the PDF is long (>10 pages), use the pages parameter to read specific sections
- Combine extracted highlights with any raw notes provided by the user

### Content Preservation (IMPORTANT)

- **User's notes are the primary axis of organization** - do NOT summarize or condense them
- Keep all substantive content from user's notes; only simplify redundant wording
- Maintain technical terms, specific phrases, and excerpts exactly as written
- Summary section is for reference's overall thesis; Notes section preserves user's highlights

### Organization Principles

- Group related points under common themes
- Use hierarchical bullet points (2-3 levels max)
- Order points by logical flow or importance, not by appearance in source
- Use LaTeX math expressions (`$...$` / `$$...$$`) whenever formulas convey information more concisely than prose; do not hesitate to include equations from the source material

### Tag Selection

- Check existing tags in `tags/` directory first
- Select 2-5 relevant tags
- When proposing a new tag, format as `[[:new-tag-name]]` and note it's a proposal
- Use `[[::continued]]` tag if the note is incomplete or will be continued later

### Language

- **Write the Summary and Notes sections in the same language as the reference material** — if the reference is in English, write in English; if in Japanese, write in Japanese
- When reorganizing excerpts and highlights from the reference, preserve the original wording as closely as possible; do not paraphrase or translate into a different language
- User's own annotations (e.g., personal commentary in a different language from the reference) should be kept in their original language as-is
- Keep technical terms in their original language when appropriate
- Do not expand abbreviations or acronyms unless they are ambiguous in context
