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
        ("420", "automated tests, all green", 0, "Linux, macOS, Windows in CI. FreeBSD and Haiku run them at release."),
        ("0", "files lost with 8 writers at once", 0, "1 write accepted, 7 refused on hash conflict. Automated test."),
        ("21 ms", "to search 2,000 notes", 0, "Tantivy index, warm. First build 139 ms. Synthetic vault."),
        ("7 MB", "one binary, nothing else to install", 0, "No Python. No Node. No C library. No server. No account.")]
body = ""
for i, (big, label, pct, note) in enumerate(rows):
    y = 130 + i * 78
    body += t(100, y + 34, big, 52, 700, RED) + t(330, y + 8, label, 15, 700) + t(330, y + 28, note, 12.5, 300, GRAY)
    if pct: body += f'<rect x="330" y="{y+38}" width="500" height="6" fill="#EEE"/><rect x="330" y="{y+38}" width="{500*pct/100:.0f}" height="6" fill="{INK}"/>'
    body += f'<line x1="100" y1="{y+60}" x2="{W-36}" y2="{y+60}" stroke="#DDD"/>' if i < 4 else ""
body += t(100, 538, "Measured 2026-11-12 with smug 1.1.0. Run them yourself: cargo test, smug bench.", 12, 300, GRAY)
save("card-stats.svg", page(568, "By the numbers", "1.1", body))

# ---------- 1.8 compare ----------
cols = ["smugBrain", "avenoxbeyin", "obsidian-2nd", "hafiza-os", "Graphiti", "mem0"]
rows = [("Notes are plain Markdown files", ["Yes", "Yes", "Yes", "Yes", "No: graph DB", "No: vector DB"]),
        ("License", ["Public domain", "MIT", "MIT", "MIT", "Apache-2.0", "Apache-2.0"]),
        ("Needs a database or account", ["No", "No", "No", "Optional Mem0", "Graph DB", "Sign-up or DB"]),
        ("Runtime to install", ["1 binary", "Python", "Python", "Python", "Python", "Python"]),
        ("Runs on FreeBSD and Haiku", ["Yes", "\u2014", "\u2014", "\u2014", "\u2014", "\u2014"]),
        ("Secret scan before every backup", ["5, always on", "opt-in", "\u2014", "\u2014", "\u2014", "\u2014"]),
        ("Stale decisions never returned", ["Yes", "\u2014", "Yes", "Yes", "Yes", "\u2014"]),
        ("Impact preview before a change", ["Yes", "\u2014", "\u2014", "\u2014", "\u2014", "\u2014"]),
        ("Eval gate before a change", ["Yes", "\u2014", "\u2014", "\u2014", "\u2014", "\u2014"])]
x0, cw = 308, 92
body = ""
for j, c in enumerate(cols):
    x = x0 + j * cw
    if j == 0:
        body += f'<rect x="{x-4}" y="100" width="{cw}" height="{70+len(rows)*54}" fill="{RED}"/>'
    body += t(x + cw / 2 - 8, 148, c if len(c) < 12 else c, 11.5, 700, PAPER if j == 0 else INK, "middle")
for i, (lab, vals) in enumerate(rows):
    y = 186 + i * 54
    body += f'<line x1="100" y1="{y-18}" x2="{W-36}" y2="{y-18}" stroke="#DDD"/>' + t(100, y + 8, lab, 12.5, 700)
    for j, v in enumerate(vals):
        x = x0 + j * cw + cw / 2 - 8
        body += t(x, y + 8, v, 11.5, 700 if j == 0 else 300, PAPER if j == 0 else (GRAY if v.startswith(("—",)) else INK), "middle")
body += t(100, 716, "Checked 2026-11-12: licenses via the GitHub API, the rest from each project's README. \"—\" = not found there (may live in code or other docs).", 11, 300, GRAY)
body += t(100, 738, "Where they lead today: community size, live-tested agent integrations (hafiza-os: Claude Code, Codex, Antigravity), built-in semantic search,", 11, 700, INK)
body += t(100, 754, "and test volume (avenoxbeyin reports 2,316; smug has 420, hafiza-os reports 360). smug --deep hands semantic queries to QMD.", 11, 700, INK)
save("card-compare.svg", page(810, "How we compare", "1.2", body))


# ---------- badges (our own, not shields.io) ----------
import re as _re
VERSION = _re.search(r'^version = "([^"]+)"', (A.parent / "Cargo.toml").read_text(), _re.M).group(1)
TESTS = 420  # update with every release: cargo test (Unix)
badges = [("release", f"v{VERSION}", True), ("tests", f"{TESTS} passing", False), ("license", "public domain", True),
          ("written in", "Rust", False), ("tested on", "Linux macOS Windows FreeBSD Haiku", False), ("speaks", "MCP", True),
          ("needs", "no account, no cloud", False)]
def one_badge(slug, lab, val, hot):
    lw, vw = 12 + len(lab) * 6.6, 14 + len(val) * 6.9
    w = lw + vw
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w:.0f} 26" width="{w:.0f}" height="26" font-family="{FONT}" role="img" aria-label="{esc(lab)}: {esc(val)}">'
           f'<rect width="{lw:.0f}" height="26" fill="{INK}"/>' + t(lw / 2, 17.5, lab, 11, 700, PAPER, "middle")
           + f'<rect x="{lw:.0f}" y=".5" width="{vw - .5:.0f}" height="25" fill="{RED if hot else PAPER}" stroke="{INK}"/>'
           + t(lw + vw / 2, 17.5, val, 11, 700, PAPER if hot else INK, "middle") + "</svg>\n")
    save(f"badge-{slug}.svg", svg)

for slug, lab, val, hot in [("release", "release", f"v{VERSION}", True), ("tests", "tests", f"{TESTS} passing", False), ("license", "license", "public domain", True),
                            ("rust", "written in", "Rust", False), ("mcp", "speaks", "MCP", True), ("nocloud", "needs", "no account, no cloud", False)]:
    one_badge(slug, lab, val, hot)

# ---------- operating system icons (real logos, one file each) ----------
LOGOS = Path(__file__).resolve().parent / "logos"
for slug, f in [("windows", "si9-windows.svg"), ("macos", "si9-apple.svg"), ("linux", "si9-linux.svg"), ("freebsd", "si9-freebsd.svg")]:
    raw = (LOGOS / f).read_text()
    inner = re.sub(r"<title>.*?</title>", "", re.search(r"<svg[^>]*>(.*)</svg>", raw, re.S).group(1), flags=re.S)
    assert "<script" not in inner.lower()
    save(f"os-{slug}.svg", f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 40" width="40" height="40" role="img" aria-label="{slug}"><rect x=".5" y=".5" width="39" height="39" fill="#fff" stroke="{INK}"/><svg x="7" y="7" width="26" height="26" viewBox="0 0 24 24" fill="{INK}">{inner}</svg></svg>\n')

# ---------- clients ----------
def logo(fname, x, y, size=34):
    """Inline a real logo (no external reference, so it renders everywhere, GitHub included)."""
    if not fname:
        return ""
    raw = (LOGOS / fname).read_text()
    inner = re.search(r"<svg[^>]*>(.*)</svg>", raw, re.S).group(1)
    inner = re.sub(r"<title>.*?</title>", "", inner, flags=re.S)
    assert "<script" not in inner.lower()
    return f'<svg x="{x}" y="{y}" width="{size}" height="{size}" viewBox="0 0 24 24" fill="{INK}">{inner}</svg>'

# (name, logo file, tier) tier 0 = `smug agent install` sets it up, 1 = prints a snippet, 2 = same MCP command, add it yourself
tools = [("Claude Code", "lb-claudecode.svg", 0), ("Codex", "lb-codex.svg", 0), ("Cursor", "lb-cursor.svg", 0), ("Gemini CLI", "lb-gemini.svg", 0),
         ("Windsurf", "lb-windsurf.svg", 0), ("Kiro", "lb-kiro.svg", 0), ("VS Code", None, 0), ("OpenCode", "lb-opencode.svg", 0),
         ("Zed", "zedindustries.svg", 1), ("Cline", "lb-cline.svg", 1), ("Copilot", "lb-githubcopilot.svg", 2), ("JetBrains", "jetbrains.svg", 2),
         ("Roo Code", "lb-roocode.svg", 2), ("Kilo Code", "lb-kilocode.svg", 2), ("Goose", "lb-goose.svg", 2), ("Amp", "lb-amp.svg", 2),
         ("Warp", "warp.svg", 2), ("Trae", "lb-trae.svg", 2), ("Qwen Code", "lb-qwen.svg", 2), ("Antigravity", "lb-antigravity.svg", 2)]
bar = {0: RED, 1: INK, 2: "#BBB"}
body = t(100, 112, "Start a task in one tool. Continue in another. Saved lessons stay with the project.", 13, 700)
TW, TH, GAP = 98, 96, 9
for i, (name, f, tier) in enumerate(tools + [("and more", None, 3)]):
    x, y = 100 + (i % 7) * (TW + GAP), 134 + (i // 7) * (TH + 10)
    dash = ' stroke-dasharray="4 3"' if tier == 3 else ""
    body += f'<rect x="{x}" y="{y}" width="{TW}" height="{TH}" fill="{PAPER}" stroke="{INK}"{dash}/>'
    if tier < 3:
        body += f'<rect x="{x}" y="{y}" width="{TW}" height="5" fill="{bar[tier]}"/>'
        body += logo(f, x + (TW - 34) / 2, y + 20) if f else t(x + TW / 2, y + 45, "{ }", 24, 700, INK, "middle")
        body += t(x + TW / 2, y + 76, name, 12, 700, INK, "middle")
        body += t(x + TW / 2, y + 89, "live-tested", 9, 700, RED, "middle") if name in ("Codex", "OpenCode") else ""
    else:
        body += t(x + TW / 2, y + 48, "+ more", 17, 700, RED, "middle") + t(x + TW / 2, y + 68, "any MCP tool", 11, 300, GRAY, "middle")
ly = 134 + 3 * (TH + 10) + 18
for k, (c, txt) in enumerate([(RED, "one command sets it up"), (INK, "prints a snippet to paste"), ("#BBB", "same command, add it in the tool")]):
    lx = 100 + k * 250
    body += f'<rect x="{lx}" y="{ly-9}" width="18" height="9" fill="{c}"/>' + t(lx + 26, ly, txt, 11.5, 700)
body += t(100, ly + 34, "smug agent install detected", 15, 700, RED) + t(318, ly + 34, "connects every tool it finds on your computer.", 13, 300)
body += t(100, ly + 60, "Any tool that can run an MCP server works with the command  smug mcp.", 12, 300, GRAY)
body += t(100, ly + 78, "Live-tested (one real session each, Linux): Codex CLI 0.167.2, OpenCode 2.2.7. All setup tools: config-edit tests and a scripted MCP client.", 11, 300, GRAY)
body += t(100, ly + 94, "Hooks (session start, before-edit reminders, capture) are installed for Claude Code only. Logos identify compatibility, not endorsement.", 11, 300, GRAY)
save("card-clients.svg", page(ly + 120, "Works with", "1.0", body))
