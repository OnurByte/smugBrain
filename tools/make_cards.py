#!/usr/bin/env python3
"""Generate the README cards (assets/*.svg) in NASA Graphics Standards (NHB 1430.2, 1976) style:
white page, black Helvetica, one warm red, hairline rules, section numbers, punch-hole margin."""
import re
from pathlib import Path

A = Path(__file__).resolve().parents[1] / "assets"
W = 880
INK, RED, GRAY, PAPER = "#111111", "#D92B2F", "#8A877E", "#FFFFFF"
FONT = "Helvetica Neue, Helvetica, Arial, sans-serif"
ICON_PATH = re.search(r' d="([^"]+)"', (A / "icon.svg").read_text()).group(1)  # 1060x845 viewBox


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;")


def t(x, y, s, size=14, weight=300, fill=INK, anchor="start", extra=""):
    return f'<text x="{x}" y="{y}" font-size="{size}" font-weight="{weight}" fill="{fill}" text-anchor="{anchor}" {extra}>{esc(s)}</text>'


def page(h, title, num, body, code="SBH 1.0"):
    holes = "".join(f'<circle cx="34" cy="{y}" r="9" fill="{INK}"/>' for y in range(30, h - 10, 50))
    head = (t(100, 46, title, 15, 700) + t(W - 36, 46, code, 12, 300, anchor="end")
            + t(W - 36, 62, "November 2026", 12, 300, anchor="end")
            + f'<line x1="100" y1="84" x2="{W-36}" y2="84" stroke="{INK}" stroke-width="1"/>')
    foot = t(W - 36, h - 18, num, 12, 300, anchor="end")
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {h}" width="{W}" height="{h}" '
           f'font-family="{FONT}" role="img" aria-label="{esc(title)}">'
           f'<rect width="{W}" height="{h}" fill="{PAPER}"/><rect x=".5" y=".5" width="{W-1}" height="{h-1}" fill="none" stroke="#DDD"/>'
           f'{holes}{head}{body}{foot}</svg>\n')
    return svg


def save(name, svg):
    (A / name).write_text(svg)


# ---------- banner ----------
b = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} 300" width="{W}" height="300" font-family="{FONT}" role="img" aria-label="smugBrain">'
     f'<rect width="{W}" height="300" fill="{PAPER}"/><rect x=".5" y=".5" width="{W-1}" height="299" fill="none" stroke="#DDD"/>'
     + "".join(f'<circle cx="34" cy="{y}" r="9" fill="{INK}"/>' for y in range(30, 290, 50))
     + f'<svg x="100" y="60" width="190" height="152" viewBox="0 0 1060 845"><rect width="1060" height="845" fill="#fff"/><path fill="{INK}" fill-rule="evenodd" d="{ICON_PATH}"/></svg>'
     + t(330, 142, "smugBrain", 78, 700, INK, extra='letter-spacing="-3"')
     + f'<rect x="333" y="156" width="150" height="8" fill="{RED}"/>'
     + t(330, 196, "Your brain has been replaced.", 19, 300)
     + t(330, 222, "Better than yours. Skill issue. No account, no cloud.", 19, 300)
     + t(100, 262, "Rust core  ·  Plain Markdown  ·  Public domain", 12, 700)
     + t(W - 36, 262, "SBH 1.0", 12, 300, anchor="end")
     + f'<line x1="100" y1="240" x2="{W-36}" y2="240" stroke="{INK}"/></svg>\n')
save("banner.svg", b)

# ---------- 1.2 stats ----------
rows = [("100%", "of your notes are plain Markdown", 100, "Open, copy, print, or leave any time."),
        ("21 ms", "to search 2,000 notes", 0, "Tantivy index, warm. First build 139 ms. Synthetic vault."),
        ("0", "files lost with 8 writers at once", 0, "1 write accepted, 7 refused on hash conflict. Automated test."),
        ("7 MB", "one binary, nothing else to install", 0, "No Python. No Node. No C library. No server. No account.")]
body = ""
for i, (big, label, pct, note) in enumerate(rows):
    y = 130 + i * 78
    body += t(100, y + 34, big, 52, 700, RED) + t(330, y + 8, label, 15, 700) + t(330, y + 28, note, 12.5, 300, GRAY)
    if pct: body += f'<rect x="330" y="{y+38}" width="500" height="6" fill="#EEE"/><rect x="330" y="{y+38}" width="{500*pct/100:.0f}" height="6" fill="{INK}"/>'
    body += f'<line x1="100" y1="{y+60}" x2="{W-36}" y2="{y+60}" stroke="#DDD"/>' if i < 3 else ""
body += t(100, 460, "Measured 2026-11-12 with smug 0.3.0. 19 tests run in CI on Linux, macOS and Windows.", 12, 300, GRAY)
save("card-stats.svg", page(490, "By the numbers", "1.1", body))

# ---------- 1.8 compare ----------
cols = ["smugBrain", "avenoxbeyin", "obsidian-2nd", "Graphiti", "mem0"]
rows = [("Notes are plain Markdown files", ["Yes", "Yes", "Yes", "No: graph DB", "No: vector DB"]),
        ("License", ["Public domain", "MIT", "MIT", "Apache-2.0", "Apache-2.0"]),
        ("Needs a database or account", ["No", "No", "No", "Graph DB", "Sign-up or DB"]),
        ("Runtime to install", ["1 binary", "Python", "Python", "Python", "Python"]),
        ("Secret scan before every backup", ["5, always on", "opt-in", "\u2014", "\u2014", "\u2014"]),
        ("Stale decisions never returned", ["Yes", "\u2014", "Yes", "Yes", "\u2014"]),
        ("Impact preview before a change", ["Yes", "\u2014", "\u2014", "\u2014", "\u2014"]),
        ("Eval gate before a change", ["Yes", "\u2014", "\u2014", "\u2014", "\u2014"])]
x0, cw = 330, 104
body = ""
for j, c in enumerate(cols):
    x = x0 + j * cw
    if j == 0:
        body += f'<rect x="{x-4}" y="100" width="{cw}" height="{70+len(rows)*54}" fill="{RED}"/>'
    body += t(x + cw / 2 - 8, 148, c if len(c) < 12 else c, 11.5, 700, PAPER if j == 0 else INK, "middle")
for i, (lab, vals) in enumerate(rows):
    y = 186 + i * 54
    body += f'<line x1="100" y1="{y-18}" x2="{W-36}" y2="{y-18}" stroke="#DDD"/>' + t(100, y + 8, lab, 13, 700)
    for j, v in enumerate(vals):
        x = x0 + j * cw + cw / 2 - 8
        body += t(x, y + 8, v, 11.5, 700 if j == 0 else 300, PAPER if j == 0 else (GRAY if v.startswith(("—",)) else INK), "middle")
body += t(100, 650, "Checked 2026-11-12: licenses via the GitHub API, the rest from each project's README. \"—\" = not found there (may live in code or other docs).", 11, 300, GRAY)
body += t(100, 670, "Where they beat us today: community size, agent count, built-in semantic search (ours needs QMD: smug search --deep).", 11, 700, INK)
save("card-compare.svg", page(700, "How we compare", "1.2", body))
