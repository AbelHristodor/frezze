# Frezze - GitHub Repository Freeze Bot

Frezze is a GitHub App built in Rust that manages repository freezes through comment commands. It uses PostgreSQL for data storage and provides CLI commands for managing freezes and running the server.

Always reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.

## Working Effectively

### Prerequisites and Environment Setup
- Rust 1.70+ is required (rustc 1.89.0+ is available in this environment)
- PostgreSQL is required for data storage
- Docker is available for infrastructure
- sqlx-cli must be installed for database migrations

### Initial Setup Commands (NEVER CANCEL - Follow ALL steps)
Run these commands in order for a fresh setup:

1. **Start Infrastructure** (takes ~17 seconds, NEVER CANCEL):
   ```bash
   make infrastructure-up
   ```
   - Sets timeout to 3+ minutes for safety
   - Downloads PostgreSQL Docker image if needed
   - Starts PostgreSQL on port 5432 with health checks

2. **Install SQLX CLI** (takes ~3.5 minutes, NEVER CANCEL):
   ```bash
   # Check if already installed
   sqlx --version || make sqlx-cli
   ```
   - Set timeout to 5+ minutes
   - Note: Make script exits with error 1 but installation succeeds

3. **Configure Environment**:
   ```bash
   cp .env.example .env
   ```
   - Edit .env if needed for custom database URLs or GitHub credentials

4. **Run Database Migrations** (takes <1 second):
   ```bash
   make migrate
   ```

### Build and Test Commands (NEVER CANCEL - Set proper timeouts)

- **Check Code** (takes ~2 minutes on first run, NEVER CANCEL):
  ```bash
  cargo check
  ```
  - Set timeout to 3+ minutes
  - Faster on subsequent runs (~3 seconds)

- **Build Application** (takes ~2 minutes on first run, NEVER CANCEL):
  ```bash
  cargo build
  # OR
  make build
  ```
  - Set timeout to 3+ minutes
  - Faster on subsequent runs (~3 seconds)

- **Run Tests** (takes <1 second):
  ```bash
  cargo test
  ```
  - 20 unit tests, all should pass
  - Set timeout to 1+ minute for safety

- **Run Linter** (takes ~4 seconds):
  ```bash
  cargo clippy
  ```
  - May show warnings but should not fail
  - Set timeout to 1+ minute for safety

- **Check Formatting** (takes <1 second):
  ```bash
  cargo fmt --check
  ```

### Running the Application

#### CLI Commands
Test CLI functionality:
```bash
# Show main help
./target/debug/frezze --help

# Show server commands
./target/debug/frezze server --help

# Show server start options
./target/debug/frezze server start --help
```

#### Start the Server
**IMPORTANT**: Server requires GitHub App credentials to run properly:
```bash
# Will fail without proper GitHub credentials
./target/debug/frezze server start \
  --gh-app-id YOUR_APP_ID \
  --gh-private-key-path path/to/key.pem
```

The server expects:
- `GITHUB_APP_ID` (required)
- `GITHUB_APP_PRIVATE_KEY_PATH` or `GITHUB_APP_PRIVATE_KEY_BASE64` (required)
- `DATABASE_URL` (defaults to localhost PostgreSQL)

## Validation

### Manual Testing Requirements
- **ALWAYS** test CLI commands after making changes
- **ALWAYS** run `cargo test` to ensure unit tests pass
- **ALWAYS** run `cargo clippy` before committing changes
- **ALWAYS** run `cargo fmt --check` to ensure code is formatted
- Database integration requires PostgreSQL to be running

### Common Validation Steps
1. Start infrastructure: `make infrastructure-up`
2. Run migrations: `make migrate`
3. Build: `cargo build`
4. Test: `cargo test`
5. Lint: `cargo clippy`
6. Format check: `cargo fmt --check`

### Known Issues and Workarounds
- **Makefile run command**: The `make run` command is incorrect - it calls `frezze start` instead of `frezze server start`
- **SQLX installation**: `make sqlx-cli` exits with error code 1 even when installation succeeds
- **Server startup**: Requires GitHub App credentials; will fail without proper configuration
- **Docker**: No Dockerfile present despite Make commands referencing Docker build

## Key Project Structure

### Important Files and Directories
```
.
├── README.md                 # Main documentation
├── Cargo.toml               # Rust dependencies and metadata
├── Makefile                 # Build automation commands
├── docker-compose.yml       # PostgreSQL infrastructure
├── .env.example            # Environment variables template
├── migrations/             # Database schema migrations
├── src/
│   ├── main.rs            # Application entry point
│   ├── cli/               # Command-line interface
│   ├── database/          # Database models and operations
│   ├── freezer/           # Core freeze management logic
│   ├── github/            # GitHub API integration
│   └── server/            # Web server and webhook handlers
└── .github/
    └── workflows/         # CI/CD pipeline
```

### Core Components
- **CLI Module**: Handles command-line parsing with clap
- **Database Module**: PostgreSQL integration using sqlx
- **Freezer Module**: Core business logic for freeze management
- **GitHub Module**: GitHub API integration using octocrab
- **Server Module**: Axum web server for webhook handling

## Timing Expectations (CRITICAL - NEVER CANCEL)

### Build Times (Set timeouts accordingly)
- **Infrastructure startup**: ~6-17 seconds (set 3+ minute timeout)
- **SQLX CLI installation**: ~3.5 minutes (set 5+ minute timeout)
- **First `cargo check`**: ~2 minutes (set 3+ minute timeout)
- **First `cargo build`**: ~2 minutes (set 3+ minute timeout)
- **Subsequent builds**: ~3 seconds
- **Database migrations**: <1 second
- **Tests**: ~4 seconds first compile + <1 second run (20 tests)
- **Clippy linting**: ~1.2 minutes first run, ~4 seconds subsequent
- **Format checking**: <1 second

### Make Commands Available
```bash
make help                    # Show all available commands
make infrastructure-up       # Start PostgreSQL (17s, NEVER CANCEL)
make infrastructure-down     # Stop PostgreSQL
make sqlx-cli               # Install SQLX CLI (3.5min, NEVER CANCEL)
make migrate                # Run database migrations
make check                  # Run cargo check
make build                  # Run cargo build (2min first time, NEVER CANCEL)
```

## GitHub Integration

### Commands Supported
The application processes GitHub issue/PR comments for:
- `/freeze` - Freeze current repository
- `/freeze-all` - Freeze all repositories in organization
- `/unfreeze` - Unfreeze current repository
- `/unfreeze-all` - Unfreeze all repositories in organization
- `/freeze-status` - Show current freeze status
- `/freeze-help` - Show command help

### Duration Formats
- Simple: `2h`, `30m`, `1d`, `45s`
- ISO 8601: `PT2H30M`, `P1D`, `PT45S`

## Development Best Practices

### Before Committing
1. **ALWAYS** run: `cargo fmt`
2. **ALWAYS** run: `cargo clippy`
3. **ALWAYS** run: `cargo test`
4. **ALWAYS** ensure PostgreSQL is running for database-dependent changes

### Common Development Tasks
- **Add new freeze commands**: Modify `src/freezer/commands.rs`
- **Add GitHub API features**: Extend `src/github/` modules
- **Database schema changes**: Add new migration in `migrations/`
- **Server endpoints**: Modify `src/server/` modules

### Testing Strategy
- Unit tests are in individual modules
- Tests cover command parsing, GitHub API utilities, and core logic
- No integration tests requiring external services
- All tests should complete in under 1 second

Remember: This is a GitHub App that requires proper credentials and webhook setup for full functionality. The application can be built and tested locally, but server functionality requires GitHub App configuration.