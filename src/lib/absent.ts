// Stands in for a backend a build leaves out (vite.config.ts): the desktop
// build has no Sub20 Lite, and Lite has no mock. Never chosen at run time.

import type { Backend } from "./types";

const absent = null as unknown as Backend;
export const webBackend = absent;
export const mockBackend = absent;
