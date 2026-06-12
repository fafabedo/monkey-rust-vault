CREATE TABLE storage_provider_credentials (
  id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  provider_id                 UUID NOT NULL REFERENCES storage_providers(id) ON DELETE CASCADE,
  -- AWS S3
  aws_profile                 TEXT,
  aws_access_key_enc          TEXT,   -- AES-256-GCM encrypted, base64
  aws_secret_key_enc          TEXT,   -- AES-256-GCM encrypted, base64
  aws_region                  TEXT,
  -- Dropbox
  dropbox_access_token_enc    TEXT,   -- AES-256-GCM encrypted, base64
  -- Google Drive
  google_sa_json_enc          TEXT,   -- AES-256-GCM encrypted JSON blob, base64
  -- Local FS
  local_base_path             TEXT,   -- absolute path on server e.g. /mnt/vault
  local_server_ip             TEXT,   -- informational
  --
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(provider_id)
);
