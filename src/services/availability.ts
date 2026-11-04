/** Explicit product capabilities. UI must not invoke unimplemented services. */
export const availability = {
  deviceDetection: true,
  backup: false,
  archiveSearch: true,
  chatImport: true,
  database: true,
} as const;
