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
     + t(100, 262, "Your agents remember. Your notes stay yours.  ·  Public domain", 12, 700)
     + t(W - 36, 262, "SBH 1.0", 12, 300, anchor="end")
     + f'<line x1="100" y1="240" x2="{W-36}" y2="240" stroke="{INK}"/></svg>\n')
save("banner.svg", b)

# ---------- 1.1 what ----------
cols = [("01", "Write", ["Your notes are plain text files", "in a folder you choose."], "Open them in any editor, Obsidian too."),
        ("02", "Remember", ["Every AI session leaves", "a short trail."], "The next session picks it up."),
        ("03", "Recall", ["Ask a question, get the", "right note back."], "Searches words and meaning.")]
body = t(100, 112, "Three verbs. That is the whole product.", 13, 300)
for i, (n, h, ls, g) in enumerate(cols):
    x = 100 + i * 255
    body += t(x, 190, n, 64, 700, RED) + t(x, 228, h, 22, 700)
    body += f'<line x1="{x}" y1="242" x2="{x+225}" y2="242" stroke="{INK}"/>'
    body += t(x, 266, ls[0], 13, 700) + t(x, 284, ls[1], 13, 700) + t(x, 308, g, 12, 300, GRAY)
body += t(100, 356, "No app to log into. Delete the tool and your notes are still there.", 13, 700)
save("card-what.svg", page(390, "What is it", "1.1", body))

# ---------- 1.2 stats ----------
rows = [("100%", "of your notes are plain Markdown", 100, "Open, copy, print, or leave any time."),
        ("5/5", "built-in self-tests pass", 100, "Run  python3 .claude/scripts/selftest.py  on the reference vault."),
        ("5", "secret scanners run before every backup", 100, "Private keys, tokens, API keys, passwords, recovery phrases."),
        ("0", "accounts or subscriptions needed", 0, "Nothing to sign up for. Nothing to cancel.")]
body = ""
for i, (big, label, pct, note) in enumerate(rows):
    y = 130 + i * 78
    body += t(100, y + 34, big, 52, 700, RED if i != 3 else INK) + t(290, y + 8, label, 15, 700) + t(290, y + 28, note, 12.5, 300, GRAY)
    body += f'<rect x="290" y="{y+38}" width="540" height="6" fill="#EEE"/><rect x="290" y="{y+38}" width="{540*pct/100:.0f}" height="6" fill="{INK}"/>'
    body += f'<line x1="100" y1="{y+60}" x2="{W-36}" y2="{y+60}" stroke="#DDD"/>' if i < 3 else ""
body += t(100, 460, "Measured on the author's reference vault, 2026-11-12. Re-run the commands yourself.", 12, 300, GRAY)
save("card-stats.svg", page(490, "By the numbers", "1.2", body))

# ---------- 1.3 targets ----------
tg = [("0", "files lost", "Two agents write at once. Nothing disappears."),
      ("0", "secrets leaked", "Keys and recovery phrases never reach an index, log, or remote."),
      ("<250 ms", "to find a note", "Target p95 for everyday keyword search. Not measured yet."),
      ("100", "test questions", "Benchmark size we want before claiming anything. Today: 12.")]
body = t(100, 120, "These are goals, not results. We will publish numbers when we have them.", 13, 700)
for i, (big, lab, note) in enumerate(tg):
    x = 100 + (i % 2) * 380; y = 150 + (i // 2) * 120
    body += f'<rect x="{x}" y="{y}" width="340" height="100" fill="none" stroke="{INK}"/>'
    body += t(x + 16, y + 52, big, 38, 700, RED) + t(x + 16, y + 74, lab, 14, 700) + t(x + 16, y + 92, note, 11, 300, GRAY)
save("card-targets.svg", page(410, "Targets (not results yet)", "1.3", body))

# ---------- 1.4 how ----------
def box(x, y, w, h, title, sub, red=False):
    f = RED if red else INK
    return (f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{f if red else PAPER}" stroke="{INK}"/>'
            + t(x + w / 2, y + 28, title, 14, 700, PAPER if red else INK, "middle")
            + t(x + w / 2, y + 48, sub, 11.5, 300, PAPER if red else GRAY, "middle"))

body = (box(100, 120, 170, 70, "Your agents", "Claude · Codex · Hermes")
        + box(355, 120, 170, 70, "smug CLI", "hooks · recall · capture", True)
        + box(610, 120, 170, 70, "Markdown vault", "the only source of truth")
        + f'<path d="M270 155H355M525 155H610" stroke="{INK}" stroke-width="2" fill="none"/>'
        + box(355, 250, 170, 70, "Search", "words → meaning → rerank")
        + box(610, 250, 170, 70, "SQLite state", "history · can be rebuilt")
        + f'<path d="M440 190V250M695 190V250" stroke="{INK}" stroke-width="2" fill="none"/>'
        + t(100, 262, "Rule 1", 13, 700, RED) + t(100, 282, "Notes are files.", 13) + t(100, 300, "Everything else", 13) + t(100, 318, "can be deleted", 13) + t(100, 336, "and rebuilt.", 13)
        + t(100, 392, "Search is tiered: exact name first, fast keywords second, heavy semantic model only when needed.", 12.5, 300, GRAY))
save("card-how.svg", page(430, "How it fits together", "1.4", body))

# ---------- 1.5 roadmap ----------
ph = [("Baseline", "Python Imperium 2.0 · running today", 100),
      ("01", "Measure: benchmark and safety tests", 0),
      ("02", "Rust core: one binary, crash-safe edits", 0),
      ("03", "Retrieval V3: three-level search", 0),
      ("04", "Decisions: why we chose what we chose", 0),
      ("05", "Ideas and capture: URL, repo, file, video", 0),
      ("06", "Shadow-test, migrate, ship", 0)]
body = ""
for i, (n, lab, p) in enumerate(ph):
    y = 118 + i * 52
    body += t(100, y + 16, n, 16, 700, RED if p else INK) + t(190, y + 14, lab, 14, 700 if p else 300)
    body += f'<rect x="190" y="{y+24}" width="520" height="6" fill="#EEE"/><rect x="190" y="{y+24}" width="{520*p/100:.0f}" height="6" fill="{RED}"/>'
    body += t(W - 36, y + 30, f"{p}%", 20, 700, INK if p else GRAY, "end")
body += t(100, 506, "Percent = share of the phase checklist finished. Updated by hand.", 12, 300, GRAY)
save("card-roadmap.svg", page(540, "Roadmap", "1.5", body))

# ---------- 1.6 license ----------
body = (t(100, 150, "Public domain.", 52, 700, RED)
        + t(100, 190, "Copy it. Fork it. Sell it. Run it anonymously. Never ask.", 17, 300)
        + f'<line x1="100" y1="212" x2="{W-36}" y2="212" stroke="{INK}"/>'
        + t(100, 244, "Unlicense", 14, 700) + t(290, 244, "No attribution, no permission, no tracking.", 14, 300)
        + t(100, 272, "Warranty", 14, 700) + t(290, 272, "None. You own the risk, like you own the keys.", 14, 300)
        + t(100, 300, "Read", 14, 700) + t(290, 300, "LICENSE · CRYPTO-ANARCHY.md", 14, 300))
save("card-license.svg", page(340, "License", "1.6", body))
print("ok", sorted(p.name for p in A.glob("*.svg")))
