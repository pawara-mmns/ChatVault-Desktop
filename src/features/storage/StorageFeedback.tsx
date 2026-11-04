import { CircleAlert, CircleCheck } from 'lucide-react';
import type { StorageFailure } from '../../services/storage';

export function StorageFeedback({
  error,
  success,
}: {
  error: StorageFailure | null;
  success: string | null;
}) {
  return (
    <>
      {error && (
        <div className="storage-feedback error-text" role="alert">
          <CircleAlert size={17} aria-hidden="true" />
          <p>{error.message}</p>
        </div>
      )}
      {success && (
        <div className="storage-feedback text-primary" role="status">
          <CircleCheck size={17} aria-hidden="true" />
          <p>{success}</p>
        </div>
      )}
    </>
  );
}
