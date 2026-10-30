#!/bin/sh
# Writes the release notes for version $1 to stdout: its section of
# CHANGELOG.md, then what each download is.
set -eu
ver=$1
awk -v v="$ver" 'index($0, "## " v " ") == 1 {f=1; next} f && /^## /{exit} f' CHANGELOG.md
cat <<'NOTES'
## Downloads

- **Windows**: `splitflap-clock.exe`. Download it and run it; nothing to install.
- **Linux**: `splitflap-clock-linux-x86_64`. Download it, `chmod +x` it and run it.
- **Mac**: `splitflap-clock-mac.zip`, for Apple silicon and Intel Macs. Unzip it, drag the app into Applications, and the first time right-click it and choose Open: it isn't signed by Apple. Its read-me has the rest.
- **User guide**: `splitflap-clock-user-guide.html`. Open it in any web browser.
- **User guide as PDFs**: `splitflap-clock-user-guide-dark.pdf` and `splitflap-clock-user-guide-light.pdf`.
- **Beer money**: `beer-money.txt`. The other Timster's Audio Works apps, and how to shout the developer a beer.
NOTES
