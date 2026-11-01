# Bridgecoil user guide: style guide

Not published. Read it before you write a page.

1. The code is the truth. Describe what the app does, verified in `src/`, `src-tauri/` and
   `crates/`. If you can't verify something, leave it out.
2. Write for one person who owns a TV and isn't an expert. Every page answers: what is this, when
   would I use it, how do I do it, what can go wrong.
3. Title pages after what the reader wants to do, not after the screen.
4. Plain second person, short sentences, active voice, present tense.
5. Use the app's exact labels, in bold. Quote its messages exactly.
6. Never play down what runs on a TV. Say what it is, where it lives, what it can do and how to
   remove it.
7. Follow the humanizer rules: no em or en dashes in prose, no "not X but Y" contrasts, no one-line
   closers, no sales words, no decorative bold.
8. Where a picture would help, add a screenshot placeholder. `scripts/docs-shots.py` turns those
   into the shot list:

       !!! screenshot "tools-overview.png"
           Tools on a connected TV, with all 11 tools on one page.

9. Run `python3 scripts/docs-shots.py` and `mkdocs build --strict` before pushing.
