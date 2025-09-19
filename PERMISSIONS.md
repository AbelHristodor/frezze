# Permission System

Frezze implements a comprehensive permission system to control which users can execute which freeze commands. The system supports both YAML configuration files and database storage for maximum flexibility.

## Role-Based Access Control

### Roles

- **Admin**: Full access to all commands
- **Maintainer**: Access to freeze/unfreeze operations based on permission flags
- **Contributor**: Read-only access (status command only)

### Command Permissions

| Command | Admin | Maintainer* | Contributor |
|---------|-------|-------------|-------------|
| `/freeze` | ✅ | ✅ (if can_freeze) | ❌ |
| `/freeze-all` | ✅ | ✅ (if can_freeze) | ❌ |
| `/unfreeze` | ✅ | ✅ (if can_unfreeze) | ❌ |
| `/unfreeze-all` | ✅ | ✅ (if can_unfreeze) | ❌ |
| `/freeze-status` | ✅ | ✅ | ✅ |
| `/schedule-freeze` | ✅ | ✅ (if can_freeze) | ❌ |

*Maintainer permissions depend on the `can_freeze` and `can_unfreeze` flags in their configuration.

## Configuration Methods

### 1. YAML Configuration (Recommended)

Create a YAML file with user permissions:

```yaml
installations:
  "12345":  # Your GitHub App installation ID
    installation_id: "12345"
    
    # Default permissions for unlisted users
    default_permissions:
      role: contributor
      can_freeze: false
      can_unfreeze: false
    
    # Global users (apply to all repositories)
    global_users:
      admin_user:
        role: admin
        can_freeze: true
        can_unfreeze: true
        can_emergency_override: true
    
    # Repository-specific permissions
    repositories:
      "owner/repo":
        repository: "owner/repo"
        users:
          maintainer_user:
            role: maintainer
            can_freeze: true
            can_unfreeze: true
```

### 2. Database Storage

User permissions can also be stored directly in the PostgreSQL database using the `permission_records` table.

## Permission Priority

The system checks permissions in this order (highest to lowest priority):

1. **Repository-specific user permissions** - Overrides all other settings
2. **Global user permissions** - Applies to all repositories in the installation
3. **Default permissions** - Fallback for the installation
4. **Denied** - If no configuration is found

## Usage

### Server Startup

```bash
# Start server with YAML configuration
./frezze server start --user-config permissions.yaml

# Or use environment variable
export USER_PERMISSIONS_CONFIG=permissions.yaml
./frezze server start
```

### Example Configuration

See `permissions.example.yaml` for a complete example with documentation.

### Database Population

You can populate the database from a YAML configuration:

```rust
use frezze::permissions::PermissionPopulator;
use frezze::config::UserPermissionsConfig;

let config = UserPermissionsConfig::load_from_file("permissions.yaml")?;
let populator = PermissionPopulator::new(database);
let count = populator.populate_from_config(&config, true).await?;
println!("Populated {} permission records", count);
```

## Error Handling

When a user attempts to execute a command they don't have permission for, Frezze will:

1. Check their permissions using the priority system above
2. Return a user-friendly error message if denied
3. Log the attempt for audit purposes

Example denied access message:
```
## ❌ Permission Denied

🚫 **Access denied for user `username`**

**Reason**: User role 'contributor' does not have freeze permissions

*Contact your repository administrator to request access.*
```

## Security Considerations

- Permissions are checked before every command execution
- Users without any configured permissions are denied by default
- Admin roles should be granted sparingly
- The system supports audit logging for compliance requirements
- Configuration files should be stored securely and version controlled

## Migration from Database

If you're already using database-stored permissions, you can continue using them. The YAML configuration system is complementary and takes priority when both are available.