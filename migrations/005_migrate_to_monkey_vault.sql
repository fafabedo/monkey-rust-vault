-- Moves all vault tables from public into the monkey_vault schema.
-- Safe to run on a live database: no data is copied or dropped.
-- Run this BEFORE exposing monkey_vault in Supabase API settings.

CREATE SCHEMA IF NOT EXISTS monkey_vault;

-- Recreate the enum in the new schema and migrate the column to use it.
-- (PostgreSQL enums are schema-scoped and cannot be moved with ALTER TABLE.)
CREATE TYPE monkey_vault.storage_provider_type AS ENUM ('s3', 'dropbox', 'google_drive', 'local');

ALTER TABLE public.storage_providers
  ALTER COLUMN type TYPE monkey_vault.storage_provider_type
  USING type::text::monkey_vault.storage_provider_type;

DROP TYPE public.storage_provider_type;

-- Move tables in dependency order so FK references are never broken mid-flight.
ALTER TABLE public.storage_providers             SET SCHEMA monkey_vault;
ALTER TABLE public.storage_provider_credentials  SET SCHEMA monkey_vault;
ALTER TABLE public.storage_buckets               SET SCHEMA monkey_vault;
ALTER TABLE public.storage_files                 SET SCHEMA monkey_vault;
