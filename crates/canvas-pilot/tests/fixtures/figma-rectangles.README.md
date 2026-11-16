# Figma SVG clipboard sample

Supplied by the user on 2026-11-15 as Figma clipboard text. The fixture removes
chat Markdown escaping before XML tags and restores the Markdown-linked namespace
to the literal `http://www.w3.org/2000/svg`. Geometry and element order are unchanged.
The Figma version, platform and exact copy command were not supplied.

Expected result: two 640 × 360 rectangles, at (0, 0) and (1040, 0), in a
1680 × 360 SVG viewport. Neither rectangle is a background. This validates SVG
text through the paste parser, not Figma's native binary/HTML clipboard formats
or live operating-system clipboard delivery.
