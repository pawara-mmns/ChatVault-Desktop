/** Explicit product capabilities. UI must not invoke unimplemented services. */
export const availability = {
  deviceDetection: true,
  backup: true,
  archiveSearch: true,
  chatImport: true,
  database: true,
} as const;
