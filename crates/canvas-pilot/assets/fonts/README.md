# Embedded open fonts

Both files are unmodified upstream font binaries under the [SIL Open Font License 1.1](OFL.txt).
Their embedded copyright and license records agree with the included notice.
They were copied from the installed Noto font packages; no runtime system-font
lookup or font download is required. This directory contains intentional source
assets, not application build output.

| File | Version | Upstream | SHA-256 |
| --- | --- | --- | --- |
| `NotoSans-Regular.ttf` | 2.015 | [Noto Latin/Greek/Cyrillic](https://github.com/notofonts/latin-greek-cyrillic) | `478c558ea716033cd60c03438f628dfa75694dcf6b5f6d505a2f05fd2b4f3823` |
| `NotoSansCJK-Regular.ttc` | 2.004 | [Noto CJK Sans 2.004](https://github.com/notofonts/noto-cjk/releases/tag/Sans2.004) | `b76b0433203017ca80401b2ee0dd69350349871c4b19d504c34dbdd80541690a` |

The TTC contains language-specific faces sharing outlines. The application uses
index 0 (JP) and 2 (SC); retain these mappings when replacing the font. Use full
fonts, not catalog-only subsets. Run the localization glyph-coverage tests and
visually check Japanese, Chinese, accented Latin and polytonic Greek after updates.
The OFL notice is also embedded and visible under Settings → Files & settings.
