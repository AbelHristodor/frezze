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

## Demo

## Commands

All commands are used in GitHub issue or PR comments:

### Basic Commands

- `/freeze` - Freeze current repository
- `/freeze-all` - Freeze all repositories in organization
- `/unfreeze` - Unfreeze current repository  
- `/unfreeze-all` - Unfreeze all repositories in organization
- `/status` - Show current freeze status
- `/unlock-pr` - Unlock a specific PR during a freeze

### Advanced Options

- `/freeze --duration 2h` - Freeze for 2 hours
- `/freeze --reason "Release v1.2.3"` - Freeze with reason
- `/freeze --duration 1d --reason "Emergency maintenance"` - Combined options
- `/schedule-freeze --from "2024-01-15T10:00:00Z" --duration 2h` - Schedule freeze
- `/status --repos repo1,repo2` - Check status for specific repositories
- `/unlock-pr --pr-number 123` - Unlock specific PR by number

### Duration Formats

- Simple: `2h`, `30m`, `1d`, `45s`
- ISO 8601: `PT2H30M`, `P1D`, `PT45S`

### PR Unlock Command

The `/unlock-pr` command allows you to temporarily unlock specific pull requests during a repository freeze, enabling them to be merged despite the freeze restrictions.

**Usage Examples:**

- `/unlock-pr` - Unlock the current PR (when used in a PR comment)
- `/unlock-pr --pr-number 123` - Unlock PR #123 (can be used from any issue/PR)

**Important Notes:**

- Only works when the repository is currently frozen
- Requires appropriate permissions (maintainer or admin role)
- The unlock remains active until the next freeze starts
- PRs are automatically refreshed with updated check run status

## Usage Examples

### Common Scenarios

**Emergency freeze during incident:**

```
/freeze --duration 2h --reason "Production incident - investigating database issues"
```

**Scheduled maintenance window:**

```
/schedule-freeze --from "2024-12-01T02:00:00Z" --duration 4h --reason "Database maintenance"
```

**Organization-wide code freeze:**

```
/freeze-all --duration 1d --reason "End-of-quarter freeze before major release"
```

**Quick status check:**

```
/status --repos frontend,backend,api
```

**Emergency PR during freeze:**

```
/unlock-pr --pr-number 456
# Comment: "Unlocking critical hotfix for production issue"
```

**Manual unfreeze:**

```
/unfreeze
# Followed by confirmation comment
```

## PR Refresh System

The PR refresh system ensures that all open pull requests have up-to-date check runs that reflect the current freeze status. This is essential for scheduled freezes and maintaining consistency.

### How It Works

1. **Scheduled Check** - The system queries for active freeze records that should be enforced
2. **PR Discovery** - For each repository with active freezes, it fetches all open pull requests
3. **Status Evaluation** - Determines if the freeze is currently active based on start/end times
4. **Check Run Update** - Creates GitHub check runs with success/failure status based on freeze state
5. **Error Handling** - Logs errors for individual PRs without stopping the entire process

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
  -e DATABASE_URL="sqlite:/app/db/frezze.db" \
  -e GITHUB_APP_ID=123456 \
  -e GITHUB_PRIVATE_KEY_PATH=/app/.privatekey.pem \
  -e GITHUB_WEBHOOK_SECRET=mysecret \
  -e OCTOFER_HOST=0.0.0.0 \
  -e OCTOFER_PORT=3000 \
  -v ./.privatekey.pem:/app/.privatekey.pem:ro \
  -v ./users.yaml:/app/users.yaml:ro \
  -p 3000:3000 \
  --restart unless-stopped \
  ghcr.io/abelhristodor/frezze:main
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
