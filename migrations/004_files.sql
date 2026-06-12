CREATE TABLE storage_files (
  id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  bucket_id        UUID NOT NULL REFERENCES storage_buckets(id),
  relative_path    TEXT NOT NULL,
  uri              TEXT NOT NULL,
  file_name        TEXT NOT NULL,
  file_size        BIGINT,
  mime_type        TEXT,
  checksum_sha256  TEXT,
  upload_status    TEXT NOT NULL DEFAULT 'pending',  -- pending | verified | failed
  uploaded_by      UUID,
  instance_id      UUID,
  space_id         UUID,
  provider_ref     TEXT,   -- S3 ETag, Drive file ID, Dropbox path_display, local abs path
  error_message    TEXT,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(bucket_id, relative_path)
);

CREATE INDEX idx_storage_files_uri    ON storage_files(uri);
CREATE INDEX idx_storage_files_bucket ON storage_files(bucket_id);
