/** Explicit product capabilities. UI must not invoke unimplemented services. */
export const availability = {
  deviceDetection: false,
  backup: false,
  archiveSearch: true,
  chatImport: true,
  database: true,
} as const;
