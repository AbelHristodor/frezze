-- Add branch column to freeze_records table to support branch-based freezes
-- This allows freezes to target specific branches (e.g., main, develop)
-- NULL means the freeze applies to all branches (backward compatible)
ALTER TABLE freeze_records ADD COLUMN branch TEXT;

-- Add index for efficient branch lookups
CREATE INDEX idx_freeze_records_branch ON freeze_records(repository, branch, status);
