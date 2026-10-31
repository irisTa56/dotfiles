import type { Register } from "claude-code";
import { register as oneTimeBanner } from "./one-time-banner";
import { register as reviewLoop } from "./review-loop";
import { register as scheduledChrome } from "./scheduled-chrome";
import { register as subagentModel } from "./subagent-model";

// hooks.json takes one module per plugin, so this one registers each of them.
export const register: Register = (on, options) => {
  oneTimeBanner(on, options);
  reviewLoop(on, options);
  scheduledChrome(on, options);
  subagentModel(on, options);
};
