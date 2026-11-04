import type { Register } from "claude-code";
import { register as compactReminder } from "./compact-reminder";
import { register as connectorAsk } from "./connector-ask";
import { register as oneTimeBanner } from "./one-time-banner";
import { register as reviewLoop } from "./review-loop";
import { register as scheduledChrome } from "./scheduled-chrome";
import { register as subagentModel } from "./subagent-model";
import { register as wrapUpLanguage } from "./wrap-up-language";

// hooks.json takes one module per plugin, so this one registers each of them.
export const register: Register = (on, options) => {
  compactReminder(on, options);
  connectorAsk(on, options);
  oneTimeBanner(on, options);
  reviewLoop(on, options);
  scheduledChrome(on, options);
  subagentModel(on, options);
  wrapUpLanguage(on, options);
};
