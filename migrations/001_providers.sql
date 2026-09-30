CREATE SCHEMA IF NOT EXISTS monkey_vault;

CREATE TYPE monkey_vault.storage_provider_type AS ENUM ('s3', 'dropbox', 'google_drive', 'local');

CREATE TABLE monkey_vault.storage_providers (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name        TEXT NOT NULL UNIQUE,
  type        monkey_vault.storage_provider_type NOT NULL,
  is_active   BOOLEAN NOT NULL DEFAULT true,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
