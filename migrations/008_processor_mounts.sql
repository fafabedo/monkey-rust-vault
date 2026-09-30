-- Maps a named mount type (temp, queue, trash, ...) to a local path on a specific processor.
-- References public.processor so processor identity stays in the existing table.

CREATE TABLE monkey_vault.storage_processor_mounts (
  id            UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
  processor_id  BIGINT  NOT NULL REFERENCES public.processor(id) ON DELETE CASCADE,
  mount_type    TEXT    NOT NULL,  -- "temp", "queue", "trash", or any custom name
  local_path    TEXT    NOT NULL,  -- absolute path on that processor's filesystem
  description   TEXT,
  is_active     BOOLEAN NOT NULL DEFAULT true,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(processor_id, mount_type)
);

CREATE INDEX idx_processor_mounts_processor ON monkey_vault.storage_processor_mounts(processor_id);

-- Extend default privileges so service_role automatically covers new tables
ALTER DEFAULT PRIVILEGES IN SCHEMA monkey_vault
  GRANT ALL ON TABLES TO service_role;

ALTER DEFAULT PRIVILEGES IN SCHEMA monkey_vault
  GRANT SELECT ON TABLES TO anon, authenticated;

GRANT ALL ON TABLE monkey_vault.storage_processor_mounts TO service_role;
GRANT SELECT ON TABLE monkey_vault.storage_processor_mounts TO anon, authenticated;
