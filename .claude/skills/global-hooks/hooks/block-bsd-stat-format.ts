import type { Block } from "./block-bash";

// A bare `stat` given -f and then a format, options before -f included, at the
// start of the command or after a separator or a space, a space being one of the
// ASCII ones the shell splits words on. A format is told from a path by a % in
// the word after -f, and from an option by not starting with -; attached to -f,
// only a word starting with % is one. So GNU's own `stat -f <path>`,
// `stat -f -c%T` and `stat -fc%T` pass. A path such as /usr/bin/stat names its
// own stat, and a quote or `|` right before the word marks an argument to
// another command. The same text after a space inside a quoted argument, such as
// a commit message, is still caught.
const FORMAT =
  /(?:^|[\t\n\v\f\r ;&(])stat +(?:-[a-zA-Z]+ +)*-f(?:['"]?%| +['"]?(?:[^-\t\n\v\f\r '"][^\t\n\v\f\r '"]*)?%)/;

// What Claude is told when the call is blocked.
const BLOCKED = `\
Blocked: stat on PATH here is GNU stat, whose -f reports the file system and
takes no format, so the format is read as a file name and the call fails.
Write \`stat -c '%Y %s %n' file\` for mtime, size and name, or
\`stat -c '%y' file\` for a readable mtime; on macOS, /usr/bin/stat is BSD stat.
Where the command only quotes the form in text, such as a commit message,
reword that text.
`;

// Blocks BSD's `stat -f <format>` where the stat on PATH is GNU stat. GNU stat's
// -f takes no argument and reports the file system instead of the file, so the
// format is read as a file name: the call exits 1, and stdout carries the file
// system's details mixed in with any other output. Written without a space, as
// in -f%z, the format is an invalid option. Where stat is BSD stat the form is
// the right one, so the hook passes everything there.
export const blockBsdStatFormat: Block = {
  name: "block-bsd-stat-format",
  matches: (command) => FORMAT.test(command),
  reason: BLOCKED,
  gnu: "stat",
};
