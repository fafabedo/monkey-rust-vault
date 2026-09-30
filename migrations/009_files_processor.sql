-- Tracks which processor wrote a local file.
-- NULL for cloud uploads (s3, dropbox, drive) — only set for processor-local schemes.

ALTER TABLE monkey_vault.storage_files
  ADD COLUMN processor_id BIGINT REFERENCES public.processor(id);

CREATE INDEX idx_storage_files_processor ON monkey_vault.storage_files(processor_id);
