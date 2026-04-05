const isDev = import.meta.env.DEV;

export const logger = {
  error(message: string, ...args: unknown[]) {
    if (isDev) console.error(`[ERROR] ${message}`, ...args);
  },
  warn(message: string, ...args: unknown[]) {
    if (isDev) console.warn(`[WARN] ${message}`, ...args);
  },
};
