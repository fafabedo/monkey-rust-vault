CREATE TABLE storage_buckets (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  slug              TEXT NOT NULL UNIQUE,   -- used in URI: "vx-bucket-01"
  display_name      TEXT NOT NULL,
  provider_id       UUID NOT NULL REFERENCES storage_providers(id),
  s3_bucket_name    TEXT,
  dropbox_root_path TEXT,
  drive_folder_id   TEXT,
  local_sub_path    TEXT,
  instance_id       UUID,
  space_id          UUID,
  is_active         BOOLEAN NOT NULL DEFAULT true,
  created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_storage_buckets_slug ON storage_buckets(slug);
