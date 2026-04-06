---
name: pdf-figure-extractor
description: Extract figures, diagrams, tables, and code blocks from PDFs as cropped PNG images. Automatically detects visual elements by analyzing PDF drawing primitives (fill colors, vector shapes) and embedded images, then crops them at precise bounding-box coordinates. Especially effective for web-page-capture PDFs where figures are rendered as vector drawings rather than embedded raster images.
---

# PDF Figure & Code Block Extractor

Extract figures, diagrams, tables, and code blocks from a PDF and save each as a cropped PNG image.

## When to Use

- User provides a PDF and asks to extract / crop / cut out figures, diagrams, code blocks, or tables as images
- The PDF may be a scholarly paper, a web-page capture, slide export, or any document containing visual elements

## Dependencies

Use `uv run --with` for dependency resolution. All Python code in this skill should be run via:

```bash
uv run --with pymupdf --with pillow python3 << 'PYEOF'
# ... code here ...
PYEOF
```

## Workflow

### 1. Analyze PDF Structure

Open the PDF with PyMuPDF and gather structural information:

```python
import fitz

doc = fitz.open(pdf_path)
for page_num in range(len(doc)):
    page = doc[page_num]

    # Embedded raster images
    images = page.get_images(full=True)

    # Vector drawings (rectangles, paths, lines)
    drawings = page.get_drawings()

    # Text blocks with font/position metadata
    blocks = page.get_text("dict")["blocks"]
```

### 2. Detect Regions by Drawing Primitives

Web-page-capture PDFs render visual elements (code blocks, diagrams, tables) as vector drawing primitives rather than embedded raster images. This means standard image extraction (`get_images()`) misses them entirely. However, these elements typically have a distinct background fill color that differs from the page background — code blocks share one dark fill, tables another, etc. By cataloging fill colors and their rectangles, you can automatically identify and locate these elements with pixel-perfect accuracy.

```python
for d in drawings:
    rect = d.get("rect")
    fill = d.get("fill")  # RGB tuple or None
    if not rect or not fill:
        continue
    w = rect[2] - rect[0]
    h = rect[3] - rect[1]

    # Example: code blocks often share a distinct background fill color
    # and have a consistent width spanning the content area
    # Catalog unique (fill_color, width) combinations to identify element types
```

**Strategy for identifying element types:**

| Element | Typical Detection Signal |
|---------|------------------------|
| Code blocks | Distinct dark background fill, consistent width matching content area, height > ~50 |
| Tables | White or light fill, or grid of thin stroke lines |
| Diagrams / flowcharts | Clusters of small filled shapes (boxes, diamonds) with connecting stroke paths |
| Embedded images | Found via `page.get_images()` with xref; position via `page.get_image_rects(xref)` |

### 3. Catalog Unique Fill Colors

Before cropping, list all distinct fill colors and their rectangles to build a map of element types:

```python
fill_colors = {}
for d in drawings:
    rect = d.get("rect")
    fill = d.get("fill")
    if not rect or not fill:
        continue
    w, h = rect[2] - rect[0], rect[3] - rect[1]
    if w < 50 or h < 20:
        continue
    key = tuple(round(c, 3) for c in fill)
    fill_colors.setdefault(key, []).append(rect)

for color, rects in sorted(fill_colors.items()):
    print(f"Fill {color}: {len(rects)} rects")
    for r in rects[:3]:
        print(f"  y={r[1]:.0f}-{r[3]:.0f} x={r[0]:.0f}-{r[2]:.0f}")
```

### 4. Render Sections for Visual Inspection

For complex PDFs (especially tall web-page captures), render page sections and use the Read tool to visually verify which regions contain figures:

```python
zoom = 2.0
mat = fitz.Matrix(zoom, zoom)

clip = fitz.Rect(x0, y_start, x1, y_end)
pix = page.get_pixmap(matrix=mat, clip=clip)
img = Image.frombytes("RGB", [pix.width, pix.height], pix.samples)
img.save("/tmp/section_preview.png")
```

Use the Read tool on the saved preview to visually confirm element boundaries. This step is critical for figures/diagrams where the bounding box cannot be fully determined from drawing primitives alone.

### 5. Crop and Save

Once regions are identified (either automatically or confirmed visually):

```python
from PIL import Image

zoom = 2.0
mat = fitz.Matrix(zoom, zoom)

for region in regions:
    clip = fitz.Rect(region.x0, region.y0, region.x1, region.y1)
    pix = page.get_pixmap(matrix=mat, clip=clip)
    img = Image.frombytes("RGB", [pix.width, pix.height], pix.samples)
    img.save(f"{output_dir}/{region.name}.png")
```

## What Can Be Fully Automated

- **Code blocks**: Background fill rectangles give exact bounding boxes. Fully automatic, pixel-perfect.
- **Tables with distinct fill**: White or colored background rectangles. Fully automatic.
- **Embedded raster images**: Extracted via `page.get_images()` and `page.get_image_rects()`. Fully automatic.

## What Requires Visual Confirmation

- **Vector diagrams / flowcharts**: Composed of many small shapes and lines. The individual components can be detected, but the overall bounding box of the "figure" must be estimated from the cluster of shapes. Render a section preview and use the Read tool to verify before cropping.
- **Figures blending into surrounding text**: When there is no distinct background or border separating the figure from body text.

## Output

- Save images to the directory specified by the user (default: same directory as the PDF)
- Naming convention: `{prefix}_{type}{number:02d}.png` (e.g., `article_code01.png`, `article_fig02.png`)
- Report a summary table of extracted images with filenames, dimensions, and descriptions

## Example

**Input:** A 2-page web-page-capture PDF of a technical blog post containing code blocks, a flowchart, and a comparison table.

**Step 1 output (fill color catalog):**

```text
Fill (0.082, 0.118, 0.173): 8 rects   ← code blocks (dark background, width=708)
  y=6160-6628 x=92-800
  y=7362-7640 x=92-800
  ...
Fill (1.0, 1.0, 1.0): 1 rect          ← table (white background)
  y=3646-3912 x=92-800
Fill (0.122, 0.125, 0.125): 5 rects    ← diagram shapes (small dark boxes)
  y=4882-4931 x=328-500
  ...
```

**Final output (summary table reported to user):**

| # | File | Size | Content |
|---|------|------|---------|
| 1 | article_code01.png | 1416x936 | Full Zip buffer layout |
| 2 | article_code02.png | 1416x556 | Miniblock header |
| ... | ... | ... | ... |
| 9 | article_fig01.png | 1416x532 | Format comparison table |
| 10 | article_fig02.png | 860x1580 | Encoding selection flowchart |

## Tips

- Use `zoom = 2.0` (144 DPI) for good quality; increase to 3.0 for high-DPI needs
- For very tall pages (web captures), render in vertical sections of ~2000 PDF-units to keep preview images manageable
- Code block fill colors are usually consistent within a single PDF — detect one, find them all
- When the user needs to manually adjust figure crops, provide the approximate coordinates as a starting point
