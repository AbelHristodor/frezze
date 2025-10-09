# Branch-based Freeze Feature Implementation

## Summary

This implementation adds support for branch-specific freezes to the Frezze GitHub repository freeze bot. Users can now freeze only PRs targeting a specific branch (e.g., `main`, `develop`), while development in other feature branches continues unaffected.

## Key Changes

### 1. Database Schema
- **Migration**: `20251009132240_add_branch_to_freeze.sql`
  - Added `branch` column (TEXT, nullable) to `freeze_records` table
  - Created index `idx_freeze_records_branch` for efficient lookups
  - NULL values mean the freeze applies to all branches (backward compatible)

### 2. Data Model
- **FreezeRecord** (`src/database/models.rs`)
  - Added `branch: Option<String>` field
  - Updated constructors `new()` and `new_scheduled()` to accept branch parameter

### 3. Command Interface
- **FreezeArgs** (`src/freezer/commands.rs`)
  - Added `--branch` flag: optional branch name to restrict freeze
- **UnfreezeArgs**
  - Added `--branch` flag: optional branch name to unfreeze specific branch
- **ScheduleFreezeArgs**
  - Added `--branch` flag: optional branch name for scheduled freezes

### 4. Command Examples
```bash
# Freeze only PRs targeting main branch
/freeze --branch main --duration 2h --reason "Production deployment"

# Freeze main branch across all repositories
/freeze-all --branch main --duration 4h

# Unfreeze specific branch
/unfreeze --branch main

# Schedule branch-specific freeze
/schedule-freeze --from "2024-01-15T10:00:00Z" --duration 2h --branch main
```

### 5. Business Logic
- **FreezeManager** (`src/freezer/manager.rs`)
  - Updated `freeze()`, `freeze_all()`, `unfreeze()` to handle branch parameter
  - Branch filtering in `handle_unfreeze()` to unfreeze only matching freezes
  - All methods pass branch parameter through the call chain

### 6. PR Filtering
- **PrRefreshService** (`src/freezer/pr_refresh.rs`)
  - Added `base_ref` field to `PullRequestInfo` to track PR target branch
  - `refresh_repository_prs()` filters PRs by branch when freeze is branch-specific
  - `refresh_single_pr()` checks if freeze applies to PR's base branch
  - Only PRs targeting the frozen branch are marked with failure status

## Behavior

### Branch-Specific Freeze
When `--branch main` is specified:
- Only PRs targeting `main` branch are frozen
- PRs targeting other branches (e.g., `develop`, `feature/*`) are NOT affected
- Multiple freezes can exist for different branches simultaneously

### All-Branch Freeze (Default)
When `--branch` is omitted:
- Freeze applies to ALL branches (backward compatible behavior)
- All PRs in the repository are frozen regardless of target branch

### Multiple Simultaneous Freezes
The system supports:
- Freeze on `main` branch
- Separate freeze on `develop` branch
- Different durations and reasons for each branch
- Independent unfreeze operations per branch

## Testing

All 43 existing tests pass, including new tests for:
- Branch flag parsing in freeze commands
- Branch flag parsing in unfreeze commands
- PullRequestInfo with base_ref field
- Database model with branch field

## Documentation

Updated documentation includes:
- README.md: Branch-based freeze examples and usage
- Module documentation: Command parsing examples with branch support
- Command help text: Description of --branch flag for all applicable commands

## Backward Compatibility

This implementation is fully backward compatible:
- Existing freezes without branch specification continue to work
- NULL branch values in database mean "all branches"
- Command syntax without --branch flag works as before
- No breaking changes to existing functionality

## Migration Path

For existing installations:
1. Run database migration: `sqlx migrate run`
2. Restart the application
3. Existing freezes continue working (branch=NULL means all branches)
4. Start using `--branch` flag in new freeze commands as needed
