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
- **Flexible duration** - Set freeze duration or schedule for specific times
- **Organization-wide freezes** - Freeze all repositories at once
- **Audit logging** - Track all freeze/unfreeze actions
- **Permission system** - Role-based access control
- **PR refresh system** - Automatically sync PR check runs with freeze status
- **Notifications** - Slack/Discord integration (planned)

## Commands

All commands are used in GitHub issue or PR comments:

### Basic Commands

- `/freeze` - Freeze current repository
- `/freeze-all` - Freeze all repositories in organization
- `/unfreeze` - Unfreeze current repository  
- `/unfreeze-all` - Unfreeze all repositories in organization
- `/freeze-status` - Show current freeze status
- `/freeze-help` - Show command help

### Advanced Options

- `/freeze --duration 2h` - Freeze for 2 hours
- `/freeze --reason "Release v1.2.3"` - Freeze with reason
- `/freeze --duration 1d --reason "Emergency maintenance"` - Combined options
- `/schedule-freeze --from "2024-01-15T10:00:00Z" --duration 2h` - Schedule freeze

### Duration Formats

- Simple: `2h`, `30m`, `1d`, `45s`
- ISO 8601: `PT2H30M`, `P1D`, `PT45S`

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

```bash
# Using docker-compose
docker-compose up -d

# Or using docker run
finch run --rm \
  -e DATABASE_URL="sqlite:/app/db/frezze.db" \
  -e GITHUB_APP_ID=1553098 \
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
git clone https://github.com/yourusername/frezze.git
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
2. **Command Parsing** - Extracts freeze commands from comments
3. **Permission Check** - Validates user permissions
4. **Branch Protection** - Applies/removes GitHub branch protection rules
5. **Database Logging** - Records all freeze operations
6. **PR Refresh** - Updates check runs on all open PRs to reflect freeze status
7. **Response** - Posts status update as comment
