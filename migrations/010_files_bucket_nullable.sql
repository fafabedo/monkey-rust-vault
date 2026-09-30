-- storage_files.bucket_id must be nullable to support processor-local uploads
-- (temp://, queue://, custom schemes) which have no associated bucket.

ALTER TABLE monkey_vault.storage_files
  ALTER COLUMN bucket_id DROP NOT NULL;
