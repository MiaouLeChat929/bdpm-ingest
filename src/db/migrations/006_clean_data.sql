-- Migration 006: Clean data columns
-- Adds normalized/clean columns for better sorting, searching, and display.

-- Add name_raw to drugs (preserve original name before normalization)
ALTER TABLE drugs ADD COLUMN name_raw TEXT;

-- Add sortable dosage to compositions (numeric mg equivalent)
ALTER TABLE compositions ADD COLUMN dosage_mg REAL;

-- Add clean substance name (normalized spaces)
ALTER TABLE compositions ADD COLUMN substance_name_clean TEXT;

-- Add clean labels (HTML stripped)
ALTER TABLE presentations ADD COLUMN labels_clean TEXT;

-- Indexes for ordering
CREATE INDEX IF NOT EXISTS idx_drugs_name_sort ON drugs(name);
CREATE INDEX IF NOT EXISTS idx_drugs_name_raw_sort ON drugs(name_raw);
CREATE INDEX IF NOT EXISTS idx_compo_dosage ON compositions(dosage_mg);
CREATE INDEX IF NOT EXISTS idx_compo_substance_clean ON compositions(substance_name_clean);
CREATE INDEX IF NOT EXISTS idx_gengroup_order ON generic_groups(group_id, sort_order, cis);
