# Illustrator SVG clipboard sample

Supplied by the user on 2026-11-15 as Illustrator clipboard text. The fixture
removes chat Markdown escaping, restores Markdown-linked namespace URLs to plain
URLs, and decodes chat `&#x20;` indentation as spaces. SVG values and element
order are unchanged. Illustrator version, platform and copy command were not supplied.

Expected geometry: two 1814.174 × 1020.473 rectangles at (0.354, 0.354) and
(395.791, 461.357). Both have a 0.709-unit centered stroke. Including outlines
expands each side by 0.3545, giving 1814.883 × 1021.182 bounds. The first outline
starts at (-0.0005, -0.0005); do not clamp away this fractional offset.

This validates the supplied SVG text through the paste parser, not Illustrator's
other native clipboard formats or live operating-system clipboard delivery.
