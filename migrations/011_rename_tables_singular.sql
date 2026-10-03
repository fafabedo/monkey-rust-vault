-- Renames all monkey_vault tables from plural to singular form.
-- Indexes are renamed to match.

ALTER TABLE monkey_vault.storage_providers            RENAME TO storage_provider;
ALTER TABLE monkey_vault.storage_provider_credentials RENAME TO storage_provider_credential;
ALTER TABLE monkey_vault.storage_buckets              RENAME TO storage_bucket;
ALTER TABLE monkey_vault.storage_processor_mounts     RENAME TO storage_processor_mount;
ALTER TABLE monkey_vault.storage_files                RENAME TO storage_file;

ALTER INDEX monkey_vault.idx_storage_buckets_slug       RENAME TO idx_storage_bucket_slug;
ALTER INDEX monkey_vault.idx_storage_files_uri          RENAME TO idx_storage_file_uri;
ALTER INDEX monkey_vault.idx_storage_files_bucket       RENAME TO idx_storage_file_bucket;
ALTER INDEX monkey_vault.idx_storage_files_processor    RENAME TO idx_storage_file_processor;
ALTER INDEX monkey_vault.idx_processor_mounts_processor RENAME TO idx_processor_mount_processor;
