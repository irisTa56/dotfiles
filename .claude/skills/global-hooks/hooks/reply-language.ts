import type { EngineInterface, Register } from "claude-code";

// The context, or none when the file cannot be read, with the cause logged: a
// failed hook here would take the plugin's other prompt.submit hooks,
// scheduled-chrome's guard among them, out of the dispatch with it.
const context = ($: EngineInterface): Promise<string[]> =>
  $.fs.read(`${$.plugin.root}/context/reply-language.md`).then(
    (text) => [text],
    (error) => {
      $.ui.log(`reply-language context was not attached: ${error}`);
      return [];
    },
  );

// Matches any text, the empty one included.
const ALL = /(?:)/;

// Adds the reply-language context to each prompt. A final reply in English is
// sent back by `dotfiles-local reply-language-stop`, a Stop command hook rather
// than a hook here: a managed plugin may continue classic.Stop past the user
// tier.
export const register: Register = (on) => {
  // Every prompt, notifications and hand-backs delivered into a running turn
  // included. Not classic.UserPromptSubmit, for the same reason. The matcher
  // selects every text; it is there because a plugin registers an event once
  // without one, and scheduled-chrome holds that registration.
  on("prompt.submit", { text: ALL }, async ($, e, next) =>
    next({ ...e, context: [...(e.context ?? []), ...(await context($))] }),
  );
};
