-- Adds a slug to the existing public.processor table.
-- This becomes the PROCESSOR_ID value in each monkey-vault instance's .env.

ALTER TABLE public.processor
  ADD COLUMN slug TEXT UNIQUE;

CREATE INDEX idx_processor_slug ON public.processor(slug);
