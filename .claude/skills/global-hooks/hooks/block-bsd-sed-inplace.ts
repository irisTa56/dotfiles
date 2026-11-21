import type { Block } from "./block-bash";

// A bare `sed` given -i and then an empty suffix, options before -i included,
// at the start of the command or after a separator or a space, a space being
// one of the ASCII ones the shell splits words on. A path such as /usr/bin/sed
// names its own sed, and a quote or `|` right before the word marks an argument
// to another command. The same text after a space inside a quoted argument is
// still caught.
const EMPTY_SUFFIX =
  /(?:^|[\t\n\v\f\r ;&(])sed +(?:-[a-zA-Z]+ +)*-i +(?:''|"")/;

// What Claude is told when the call is blocked.
const BLOCKED = `\
Blocked: sed on PATH here is GNU sed, whose -i takes no separate suffix
argument, so the empty string after -i is read as the script or as a file
name and the call fails. Write \`sed -i 's/.../.../' file\`, or use the Edit
tool where the edit is not mechanical.
`;

// Blocks BSD's `sed -i ''` where the sed on PATH is GNU sed. GNU sed's -i takes
// no separate suffix argument, so the empty string becomes the script and the
// real script a file name, or with -e the empty string becomes a file name;
// either way the call exits 2, and without -e it edits nothing. Where sed is BSD
// sed the form is the right one, so the hook passes everything there.
export const blockBsdSedInplace: Block = {
  name: "block-bsd-sed-inplace",
  matches: (command) => EMPTY_SUFFIX.test(command),
  reason: BLOCKED,
  gnu: "sed",
};
