# Permission System

Frezze implements a comprehensive permission system to control which users can execute freeze commands. The system uses **YAML configuration files as the single source of truth** for user permissions, providing a simple and maintainable approach to access control.

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
| `/status` | ✅ | ✅ | ✅ |
| `/schedule-freeze` | ✅ | ✅ (if can_freeze) | ❌ |
| `/unlock-pr` | ✅ | ✅ (if can_unfreeze) | ❌ |

*Maintainer permissions depend on the `can_freeze` and `can_unfreeze` flags in their configuration.

## YAML Configuration (Single Source of Truth)

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

## Permission Priority

The system checks permissions in this order (highest to lowest priority):

1. **Repository-specific user permissions** - Overrides all other settings
2. **Global user permissions** - Applies to all repositories in the installation
3. **Default permissions** - Fallback for the installation
4. **Denied** - If no configuration is found

## Usage

### Server Startup

The server **requires a YAML configuration file** to operate with permission checking:

```bash
export USER_PERMISSIONS_CONFIG=permissions.yaml
./frezze
```

**Note**: If no configuration file is provided, all commands except `/status` will be denied.

### Example Configuration

See `permissions.example.yaml` for a complete example with documentation.

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

- **Default deny policy** - Users without any configured permissions are blocked by default
- **Configuration validation** - Invalid YAML or role configurations are rejected at startup
- **Admin roles should be granted sparingly** - Full administrative access
- **Configuration files should be stored securely** and version controlled
- **Audit logging** - All permission checks are logged for compliance requirements

## Configuration Management

### Best Practices

1. **Version control** your permission configuration files
2. **Review changes** to permissions through pull requests
3. **Use descriptive user names** that match GitHub usernames exactly
4. **Test configuration** by loading it before deployment
5. **Monitor logs** for permission denied attempts

### Configuration Validation

The system validates configuration files on startup and will fail if:

- YAML syntax is invalid
- Required fields are missing
- Installation IDs don't match between keys and values
- User roles are not recognized (admin, maintainer, contributor)

This ensures configuration errors are caught early rather than at runtime.

