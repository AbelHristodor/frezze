<p align="center">
  <a href="#"><img src="https://github.com/AbelHristodor/frezze/blob/main/.github/assets/frezze.PNG?raw=true" width="160" alt="Frezze's logo" /></a>

</p>
<h3 align="center"><a href="#">Frezze</a></h3>
<p align="center">GitHub App to Freeze activity in your repo! </p>
<p align="center">
<img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/AbelHristodor/frezze/docs.yaml">
<img alt="GitHub License" src="https://img.shields.io/github/license/AbelHristodor/frezze">
<img alt="GitHub top language" src="https://img.shields.io/github/languages/top/AbelHristodor/frezze">
</p>

A GitHub App built in Rust that manages repository freezes through comment commands. Temporarily restrict repository access during deployments, maintenance, or critical operations.

Built with [Octofer](https://github.com/AbelHristodor/octofer).

## Demo

<video src="https://github.com/user-attachments/assets/a8dd0c8a-00dd-4956-87e1-8e5b947edc61" width="600px" height="400px" controls></video>

## Features

- **Comment-based commands** - Control freezes directly from GitHub issues/PRs
- **Flexible scheduling** - Set freeze duration or schedule for specific times
- **Organization-wide freezes** - Freeze all repositories at once
- **PR unlock system** - Selectively allow PRs to merge during freezes
- **Audit logging** - Track all freeze/unfreeze actions in SQLite database
- **Permission system** - Role-based access control with configurable permissions
- **PR refresh system** - Automatically sync PR check runs with freeze status
- **Multiple duration formats** - Support both simple (2h, 30m) and ISO 8601 formats
- **Real-time status updates** - Check freeze status across multiple repositories

## Commands

All commands are used in GitHub issue or PR comments:

### Basic Commands

- `@frezze freeze` - Freeze current repository
- `@frezze freeze --repo owner/repo1,owner/repo2` - Freeze specific repositories
- `@frezze freeze-all` - Freeze all repositories in organization
- `@frezze freeze-all --repo owner/repo1,owner/repo2` - Freeze specific repositories
- `@frezze unfreeze` - Unfreeze current repository  
- `@frezze unfreeze-all` - Unfreeze all repositories in organization
- `@frezze status` - Show current freeze status
- `@frezze unlock-pr` - Unlock a specific PR during a freeze

### Advanced Options

- `@frezze freeze --duration 2h` - Freeze for 2 hours
- `@frezze freeze --reason "Release v1.2.3"` - Freeze with reason
- `@frezze freeze --duration 1d --reason "Emergency maintenance"` - Combined options
- `@frezze freeze --repo owner/repo1,owner/repo2 --duration 2h` - Freeze specific repos for 2 hours
- `@frezze freeze --repo owner/repo1 --repo owner/repo2` - Freeze multiple repos using separate flags
- `@frezze freeze-all --repo owner/repo1,owner/repo2` - Freeze only specific repos instead of all
- `@frezze schedule-freeze --from "2024-01-15T10:00:00Z" --duration 2h` - Schedule freeze
- `@frezze status --repos repo1,repo2` - Check status for specific repositories
- `@frezze unlock-pr --pr-number 123` - Unlock specific PR by number
- `@frezze unlock-pr --reason "emergency"` - Unlock current PR with reason
- `@frezze unfreeze --reason "Issue resolved"` - Unfreeze with reason

### Branch-based Freezes

Branch-based freezes allow you to freeze only PRs targeting a specific branch (e.g., `main`, `develop`), while development in other branches continues unaffected.

**Usage Examples:**

- `@frezze freeze --branch main` - Freeze only PRs merging into the main branch
- `@frezze freeze --branch main --duration 2h --reason "Production deployment"` - Freeze main branch for 2 hours
- `@frezze freeze-all --branch main` - Freeze main branch across all repositories
- `@frezze unfreeze --branch main` - Unfreeze only the main branch
- `@frezze schedule-freeze --from "2024-01-15T10:00:00Z" --duration 2h --branch main` - Schedule branch-specific freeze

**Important Notes:**

- When `--branch` is not specified, the freeze applies to all branches (default behavior)
- A repository can have multiple active freezes for different branches simultaneously
- Each branch freeze is tracked independently and can be unfrozen separately
- Branch-based freezes work with all freeze commands (`freeze`, `freeze-all`, `schedule-freeze`)

### Duration Formats

- Simple: `2h`, `30m`, `1d`, `45s`
- ISO 8601: `PT2H30M`, `P1D`, `PT45S`

### PR Unlock Command

The `@frezze unlock-pr` command allows you to temporarily unlock specific pull requests during a repository freeze, enabling them to be merged despite the freeze restrictions.

**Usage Examples:**

- `@frezze unlock-pr` - Unlock the current PR (when used in a PR comment)
- `@frezze unlock-pr --pr-number 123` - Unlock PR #123 (can be used from any issue/PR)
- `@frezze unlock-pr --pr-number 123 --reason "Critical security fix"` - Unlock with reason

**Important Notes:**

- Only works when the repository is currently frozen
- Requires appropriate permissions (maintainer or admin role)
- The unlock remains active until the next freeze starts
- PRs are automatically refreshed with updated check run status

## Quick Start

### Prerequisites

- Rust 1.70+
- SQLite (included with Rust build)
- GitHub App credentials

### Setup

#### Option 1: Docker (Recommended)

> [Finch](https://github.com/runfinch/finch) is a OSS client for docker. Substitute *finch* with *docker* in the commands below if you're using docker.

```bash
# Using docker/ compose
finch compose up -d 

# Or using docker run
finch run --rm \
  -e DATABASE_URL="sqlite:/frezze.db" \
  -e GITHUB_APP_ID=123456 \
  -e GITHUB_PRIVATE_KEY_PATH=/.privatekey.pem \
  -e GITHUB_WEBHOOK_SECRET=mysecret \
  -e OCTOFER_HOST=0.0.0.0 \
  -e OCTOFER_PORT=3000 \
  -v ./.privatekey.pem:/.privatekey.pem:ro \
  -v ./users.yaml:/users.yaml:ro \
  -p 3000:3000 \
  --restart unless-stopped \
  ghcr.io/abelhristodor/frezze:latest
```

#### Option 2: From Source

```bash
# Clone and build
git clone https://github.com/AbelHristodor/frezze.git
cd frezze
cargo build

# Setup database (SQLite file will be created automatically)
make migrate

# Run application
make run
```

### Configuration

Copy `.env.example` to `.env` and configure:

```env
DATABASE_URL=sqlite:frezze.db?mode=rwc
GITHUB_APP_ID=your_app_id
GITHUB_PRIVATE_KEY_PATH=path/to/private-key.pem
WEBHOOK_SECRET=your_webhook_secret
PERMISSIONS_PATH=users.yaml # check PERMISSIONS.md
PORT=3000
```

Make sure to check [PERMISSIONS.md](./PERMISSIONS.md) for more information regarding the permission system.

## Development

### Available Make Commands

```bash
make help              # Show all available commands
make build             # Build the application
make test              # Run tests
make run               # Start the server
make migrate           # Run database migrations
```

### Project Structure

```
src/
├── main.rs           # Application entry point
├── freezer/          # Core freeze management
│   ├── commands.rs   # Command parsing
│   ├── manager.rs    # Freeze operations
│   └── pr_refresh.rs # PR check run refresh system
├── github/           # GitHub API integration
├── database/         # Database models and operations
├── server/           # Web server and webhook handlers
└── config/           # Configuration management
```

## How It Works

1. **GitHub Webhook** - Receives issue/PR comment events
2. **Command Parsing** - Extracts freeze commands from comments using clap parser
3. **Permission Check** - Validates user permissions against YAML configuration
4. **Branch Protection** - Applies/removes GitHub branch protection rules
5. **Database Logging** - Records all freeze/unlock operations in SQLite
6. **PR Refresh** - Updates check runs on all open PRs to reflect freeze status
7. **PR Unlock Management** - Tracks individually unlocked PRs during freezes
8. **Response** - Posts formatted status update as GitHub comment
