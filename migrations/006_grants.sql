-- Grant PostgREST roles access to the monkey_vault schema.
-- Required after exposing the schema in Supabase API settings.

GRANT USAGE ON SCHEMA monkey_vault TO anon, authenticated, service_role;

GRANT ALL ON ALL TABLES    IN SCHEMA monkey_vault TO service_role;
GRANT ALL ON ALL SEQUENCES IN SCHEMA monkey_vault TO service_role;

-- Read-only access for anon/authenticated (tighten per table as needed)
GRANT SELECT ON ALL TABLES IN SCHEMA monkey_vault TO anon, authenticated;
