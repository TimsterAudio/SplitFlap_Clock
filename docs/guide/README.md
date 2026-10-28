# The user guide

`Split-Flap Clock User Guide.html` in the repository's root is built from
here: `body.html` (the text), `style.css` (the look, dark and light),
`fonts.css` (Barlow Semi Condensed, built in) and the board's own lettering
from `assets/`.

```sh
python3 docs/guide/build.py   # the HTML
node docs/guide/pdf.js        # the dark and light PDFs (needs Playwright)
```

Edit `body.html`, rebuild both, and commit the results with it.
