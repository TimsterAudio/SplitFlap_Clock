"""Builds the user guide, "Split-Flap Clock User Guide.html" in the repo's
root: fonts, styles and body in one self-contained file, with [[TEXT]]
turned into flap cards. [[[TEXT]]] is the same, for the big time. In a
zone row, '|' separates the columns and the name is padded to 12 cards.

    python3 docs/guide/build.py
    node docs/guide/pdf.js      # the dark and light PDFs, with Playwright
"""
import base64
import html
import pathlib
import re

here = pathlib.Path(__file__).resolve().parent
root = here.parent.parent
fonts = (here / "fonts.css").read_text()
flap = base64.b64encode((root / "assets" / "BarlowCondensed-SemiBold.ttf").read_bytes()).decode()
css = (here / "style.css").read_text()
body = (here / "body.html").read_text()


def cards(text, blanks=True):
    out = []
    for ch in text:
        if ch == ":":
            out.append('<span class="colon"></span>')
        elif ch == "|":
            out.append('<span class="gap"></span><span class="gap"></span>')
        elif ch == " ":
            out.append('<span class="fl blank">&#160;</span>' if blanks else '<span class="gap"></span>')
        else:
            out.append(f'<span class="fl">{html.escape(ch)}</span>')
    return "".join(out)


def zone(text):
    name, t, day = text.split("|")
    return cards(name.strip().ljust(12)) + cards("|" + t + "|" + day)


body = re.sub(r'(<div class="row zone">)\[\[(.*?)\]\]', lambda m: m.group(1) + zone(m.group(2)), body)
body = re.sub(r"\[\[\[(.*?)\]\]\]", lambda m: cards(m.group(1)), body)
body = re.sub(r"\[\[(.*?)\]\]", lambda m: cards(m.group(1), blanks=False), body)
assert "[[" not in body

face = (
    "@font-face{font-family:'Flap';font-style:normal;font-weight:600;font-display:block;"
    f"src:url(data:font/ttf;base64,{flap}) format('truetype')}}"
)
page = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Split-Flap Clock for Dummies</title>
<style>
{fonts}
{face}
{css}
</style>
</head>
<body>
{body}
</body>
</html>
"""
(root / "Split-Flap Clock User Guide.html").write_text(page, encoding="utf-8")
