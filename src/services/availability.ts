/** Explicit product capabilities. UI must not invoke unimplemented services. */
export const availability = {
  deviceDetection: false,
  backup: false,
  archiveSearch: false,
  chatImport: false,
  database: false,
} as const;
