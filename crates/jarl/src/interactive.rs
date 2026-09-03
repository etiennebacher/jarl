use anyhow::Result;
use colored::{Color, Colorize};
use console::{Key, Term};
use jarl_core::diagnostic::Diagnostic;
use jarl_core::fix::{FixDecision, FixPrompt};
use jarl_core::vcs::VcsStatus;
use std::io::{BufRead, Write};

/// Frame width used when the terminal size is unknown (a pipe, a test).
const DEFAULT_WIDTH: usize = 80;
const MIN_WIDTH: usize = 40;
const MAX_WIDTH: usize = 100;

/// Unchanged lines shown on each side of a fix.
const CONTEXT_LINES: usize = 3;

/// Width of the two line-number columns and the separator that follows them.
const GUTTER: usize = 12;

/// What each key does, spelled out rather than crammed into the question: the
/// key, the colour it is printed in, what it is called, and what it does. The
/// name carries its own padding, since `{:<width$}` would count the colour
/// escapes of the key next to it.
const CHOICES: [(&str, Color, &str, &str); 4] = [
    ("y", Color::Green, "accept    ", "apply this fix"),
    ("n", Color::Red, "reject    ", "leave this code as it is"),
    (
        "a",
        Color::Green,
        "accept all",
        "apply this fix and all the remaining ones",
    ),
    (
        "q",
        Color::Yellow,
        "quit      ",
        "stop here, keeping the fixes already applied",
    ),
];

/// Drives `--interactive`: previews each fix and reads the user's decision.
///
/// Holds the state that must outlive a single question — `a` (accept the rest)
/// and `q` (stop asking) — so that the core loop never has to carry a decision
/// from one file to the next.
pub struct TerminalPrompt {
    term: Term,
    /// Width the diff frame is drawn at, fixed once so every fix lines up.
    width: usize,
    /// Terminal rows the preview on screen takes up, so the next one can be
    /// drawn over it.
    drawn: usize,
    accept_all: bool,
    quit: bool,
    pub applied: usize,
    pub skipped: usize,
}

impl Default for TerminalPrompt {
    fn default() -> Self {
        let term = Term::stdout();
        let width = term
            .size_checked()
            .map_or(DEFAULT_WIDTH, |(_, cols)| cols as usize)
            .clamp(MIN_WIDTH, MAX_WIDTH);
        Self {
            term,
            width,
            drawn: 0,
            accept_all: false,
            quit: false,
            applied: 0,
            skipped: 0,
        }
    }
}

impl TerminalPrompt {
    /// Read one answer. On a real terminal this is a single keypress; when
    /// stdin is a pipe (tests, `echo y | jarl ...`) it falls back to a line, so
    /// the flow stays drivable without a tty.
    ///
    /// `default` is what Enter means. `None` is returned when there is nothing
    /// left to read, which is the one case a caller must not retry on.
    fn read_answer(&self, default: char) -> Result<Option<char>> {
        if self.term.is_term() {
            return match self.term.read_key() {
                Ok(Key::Char(c)) => Ok(Some(c.to_ascii_lowercase())),
                Ok(Key::Enter) => Ok(Some(default)),
                // Escape and Ctrl-C both mean "I'm done here".
                Ok(Key::Escape) => Ok(None),
                // Any other key (arrows, function keys) is not an answer.
                Ok(_) => Ok(Some('\0')),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Ok(None),
                Err(e) => Err(e.into()),
            };
        }

        let mut line = String::new();
        if std::io::stdin().lock().read_line(&mut line)? == 0 {
            return Ok(None);
        }
        match line.trim().chars().next() {
            Some(c) => Ok(Some(c.to_ascii_lowercase())),
            None => Ok(Some(default)),
        }
    }

    /// Ask a question until it gets an answer we understand.
    ///
    /// `keys` lists the accepted answers, the first of which is what Enter
    /// means. `on_eof` is the answer to assume when there is no one left to
    /// ask — a closed stdin, Escape, or Ctrl-C — and must be the choice that
    /// does the least, since it is taken without the user's say-so.
    fn ask_key(&mut self, question: &str, keys: &str, on_eof: char) -> Result<char> {
        let default = keys.chars().next().unwrap_or(on_eof);

        loop {
            print!("{question} ");
            std::io::stdout().flush()?;
            self.drawn += 1;

            match self.read_answer(default)? {
                // Neither a keypress nor a piped line is echoed, so close the
                // line ourselves and show what was answered.
                Some(c) if keys.contains(c) => {
                    println!("{c}");
                    return Ok(c);
                }
                None => {
                    println!("{on_eof}");
                    return Ok(on_eof);
                }
                Some(_) => {
                    let expected: Vec<String> = keys.chars().map(String::from).collect();
                    println!("?\nPlease answer with one of: {}.", expected.join(", "));
                    self.drawn += 1;
                }
            }
        }
    }

    /// Erase the preview on screen so the next one can take its place. Piped
    /// output has no cursor to move, and a preview taller than the screen has
    /// scrolled past the point where its first row can still be reached, so
    /// both are left to stack.
    pub fn clear_preview(&mut self) -> Result<()> {
        let erasable = self
            .term
            .size_checked()
            .is_some_and(|(rows, _)| self.drawn < rows as usize);
        if self.drawn > 0 && erasable {
            self.term.clear_last_lines(self.drawn)?;
        }
        self.drawn = 0;
        Ok(())
    }

    /// Print one line of a preview, counting the rows it takes once wrapped so
    /// that `clear_preview` erases exactly what was drawn.
    fn draw(&mut self, line: &str) {
        println!("{line}");
        let cols = self
            .term
            .size_checked()
            .map_or(self.width, |(_, c)| c as usize);
        self.drawn += console::measure_text_width(line).div_ceil(cols).max(1);
    }
}

impl FixPrompt for TerminalPrompt {
    fn ask(&mut self, path: &str, contents: &str, diagnostic: &Diagnostic) -> Result<FixDecision> {
        if self.quit {
            return Ok(FixDecision::Quit);
        }
        if self.accept_all {
            self.applied += 1;
            return Ok(FixDecision::Accept);
        }

        self.clear_preview()?;

        self.draw("");
        self.draw(&header(path, diagnostic));
        self.draw(&diagnostic.message.body.dimmed().to_string());
        self.draw("");
        for line in diff(contents, diagnostic, self.width).lines() {
            self.draw(line);
        }
        self.draw("");
        for (key, color, name, effect) in CHOICES {
            self.draw(&format!(
                "  {} {name}  {}",
                key.color(color).bold(),
                effect.bright_black()
            ));
        }
        self.draw("");

        match self.ask_key("Apply this fix?", "ynaq", 'q')? {
            'y' => {
                self.applied += 1;
                Ok(FixDecision::Accept)
            }
            'a' => {
                self.accept_all = true;
                self.applied += 1;
                Ok(FixDecision::Accept)
            }
            'q' => {
                self.quit = true;
                Ok(FixDecision::Quit)
            }
            _ => {
                self.skipped += 1;
                Ok(FixDecision::Skip)
            }
        }
    }

    fn confirm_vcs(&mut self, status: &VcsStatus) -> Result<bool> {
        println!();
        match status {
            VcsStatus::Clean => return Ok(true),
            VcsStatus::NoVcs => {
                println!(
                    "{}: no Version Control System (e.g. Git) was found on this project, \n\
                     so the fixes you accept cannot be reverted.",
                    "Warning".yellow().bold()
                );
            }
            VcsStatus::Dirty(files) => {
                println!(
                    "{}: this project has uncommitted changes, so the fixes you accept \n\
                     will be mixed with them:",
                    "Warning".yellow().bold()
                );
                for file in files {
                    println!("  * {file}");
                }
            }
        }
        println!();

        let confirmed = self.ask_key("Go through fixes anyway? [y/n]", "yn", 'n')? == 'y';
        // The warning above is not a preview, so nothing here gets erased.
        self.drawn = 0;
        if !confirmed {
            self.quit = true;
        }
        Ok(confirmed)
    }

    fn aborted(&self) -> bool {
        self.quit
    }
}

/// `path:row:col  rule_name`, with a marker for fixes that can change behavior.
fn header(path: &str, diagnostic: &Diagnostic) -> String {
    let (row, col) = match diagnostic.location {
        Some(loc) => (loc.row(), loc.column() + 1),
        None => (0, 0),
    };
    let tag = if diagnostic.has_unsafe_fix() {
        format!(" {}", "[unsafe]".yellow())
    } else {
        String::new()
    };
    format!(
        "{}  {}{tag}",
        format!("{path}:{row}:{col}").white(),
        diagnostic.message.rule.name().red()
    )
}

/// The fix as a framed diff: the lines it rewrites, marked `-`/`+`, with a few
/// unchanged lines around them and old/new line numbers in the gutter.
fn diff(contents: &str, diagnostic: &Diagnostic, width: usize) -> String {
    let (start, end) = (diagnostic.fix.start(), diagnostic.fix.end());

    // Widen the fix range to whole lines so the diff reads as source.
    let line_start = contents[..start].rfind('\n').map_or(0, |p| p + 1);
    let line_end = contents[end..]
        .find('\n')
        .map_or(contents.len(), |p| end + p);

    let before = &contents[line_start..line_end];
    let after = format!(
        "{}{}{}",
        &contents[line_start..start],
        diagnostic.fix.content,
        &contents[end..line_end]
    );

    let leading: Vec<&str> = {
        let mut lines: Vec<&str> = contents[..line_start]
            .lines()
            .rev()
            .take(CONTEXT_LINES)
            .collect();
        lines.reverse();
        lines
    };
    // The slice starts on the newline closing the last rewritten line, so the
    // first item it yields is empty and has to go.
    let trailing: Vec<&str> = contents[line_end..]
        .lines()
        .skip(1)
        .take(CONTEXT_LINES)
        .collect();

    // Both sides of the diff share the lines above the fix, so they also share
    // the number the frame starts at.
    let first = contents[..line_start].lines().count() + 1 - leading.len();
    let (mut old, mut new) = (first, first);

    let mut out = format!("{}\n", rule(width, '┬'));
    for line in leading {
        out.push_str(&row(Some(old), Some(new), ' ', line, |s| s.normal()));
        old += 1;
        new += 1;
    }
    for line in before.lines() {
        out.push_str(&row(Some(old), None, '-', line, |s| s.red()));
        old += 1;
    }
    for line in after.lines() {
        out.push_str(&row(None, Some(new), '+', line, |s| s.green()));
        new += 1;
    }
    for line in trailing {
        out.push_str(&row(Some(old), Some(new), ' ', line, |s| s.normal()));
        old += 1;
        new += 1;
    }
    out.push_str(&format!("{}\n", rule(width, '┴')));
    out
}

/// The horizontal frame line, with `joint` where the gutter separator meets it.
fn rule(width: usize, joint: char) -> String {
    format!(
        "{}{joint}{}",
        "─".repeat(GUTTER),
        "─".repeat(width.saturating_sub(GUTTER + 1))
    )
    .dimmed()
    .to_string()
}

/// One diff line: `old new │<sign> content`, blanking the line number that the
/// side in question doesn't have.
fn row(
    old: Option<usize>,
    new: Option<usize>,
    sign: char,
    line: &str,
    paint: impl Fn(&str) -> colored::ColoredString,
) -> String {
    let number = |n: Option<usize>| n.map_or_else(|| " ".repeat(5), |n| format!("{n:>5}"));
    format!(
        "{} {} {}{}\n",
        number(old).dimmed(),
        number(new).dimmed(),
        "│".dimmed(),
        paint(&format!("{sign}{}", display(line)))
    )
}

/// Make one source line printable: tabs expanded the way the diagnostic
/// renderer expands them, and no stray `\r` from CRLF files.
fn display(line: &str) -> String {
    line.trim_end_matches('\r').replace('\t', "    ")
}
