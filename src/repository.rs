use octofer::octocrab;

/// Represents a GitHub repository with owner and name components.
///
/// This struct ensures consistent handling of repository identifiers throughout
/// the application, providing utilities to construct and deconstruct the
/// "owner/repo" format used by the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    pub owner: String,
    pub name: String,
}

impl Repository {
    /// Creates a new Repository instance.
    ///
    /// # Arguments
    ///
    /// * `owner` - The repository owner (username or organization)
    /// * `name` - The repository name
    pub fn new(owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            name: name.into(),
        }
    }

    /// Returns the repository in "owner/repo" format.
    ///
    /// This format is used by the database and for display purposes.
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    /// Parses a "owner/repo" string into a Repository.
    ///
    /// # Arguments
    ///
    /// * `full_name` - Repository in "owner/repo" format
    ///
    /// # Returns
    ///
    /// Returns `Some(Repository)` if the format is valid, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use frezze::repository::Repository;
    ///
    /// let repo = Repository::parse("octocat/Hello-World").unwrap();
    /// assert_eq!(repo.owner, "octocat");
    /// assert_eq!(repo.name, "Hello-World");
    ///
    /// assert!(Repository::parse("invalid").is_none());
    /// ```
    pub fn parse(full_name: &str) -> Option<Self> {
        let parts: Vec<&str> = full_name.splitn(2, '/').collect();
        if parts.len() == 2
            && !parts[0].is_empty()
            && !parts[1].is_empty()
            && !parts[1].contains('/')
        {
            Some(Self::new(parts[0], parts[1]))
        } else {
            None
        }
    }

    /// Returns the owner component.
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Returns the name component.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Display for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_name())
    }
}

impl From<&Repository> for String {
    fn from(repo: &Repository) -> Self {
        repo.full_name()
    }
}

impl From<Repository> for String {
    fn from(repo: Repository) -> Self {
        repo.full_name()
    }
}

impl From<&octocrab::models::Repository> for Repository {
    fn from(repo: &octocrab::models::Repository) -> Self {
        let owner = if let Some(owner) = repo.owner.clone() {
            owner.login
        } else {
            String::new()
        };

        Self {
            owner,
            name: repo.name.clone(),
        }
    }
}

impl From<octocrab::models::Repository> for Repository {
    fn from(repo: octocrab::models::Repository) -> Self {
        let owner = if let Some(owner) = repo.owner {
            owner.login
        } else {
            String::new()
        };

        Self {
            owner,
            name: repo.name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let repo = Repository::new("octocat", "Hello-World");
        assert_eq!(repo.owner, "octocat");
        assert_eq!(repo.name, "Hello-World");
    }

    #[test]
    fn test_full_name() {
        let repo = Repository::new("octocat", "Hello-World");
        assert_eq!(repo.full_name(), "octocat/Hello-World");
    }

    #[test]
    fn test_parse_valid() {
        let repo = Repository::parse("octocat/Hello-World").unwrap();
        assert_eq!(repo.owner, "octocat");
        assert_eq!(repo.name, "Hello-World");
    }

    #[test]
    fn test_parse_with_complex_names() {
        let repo = Repository::parse("my-org/some-repo.test").unwrap();
        assert_eq!(repo.owner, "my-org");
        assert_eq!(repo.name, "some-repo.test");
    }

    #[test]
    fn test_parse_invalid() {
        assert!(Repository::parse("invalid").is_none());
        assert!(Repository::parse("").is_none());
        assert!(Repository::parse("/repo").is_none());
        assert!(Repository::parse("owner/").is_none());
        assert!(Repository::parse("owner//repo").is_none());
    }

    #[test]
    fn test_display() {
        let repo = Repository::new("octocat", "Hello-World");
        assert_eq!(format!("{}", repo), "octocat/Hello-World");
    }

    #[test]
    fn test_from_string() {
        let repo = Repository::new("octocat", "Hello-World");
        let s: String = repo.into();
        assert_eq!(s, "octocat/Hello-World");
    }

    #[test]
    fn test_accessors() {
        let repo = Repository::new("octocat", "Hello-World");
        assert_eq!(repo.owner(), "octocat");
        assert_eq!(repo.name(), "Hello-World");
    }

    #[test]
    fn test_integration_owner_repo_format() {
        // Test that the Repository struct properly handles the owner/repo format
        // that the database expects versus the separate components GitHub API uses
        let repo = Repository::new("octocat", "Hello-World");

        // Database format (what FreezeManager now expects)
        let db_format = repo.full_name();
        assert_eq!(db_format, "octocat/Hello-World");

        // GitHub API format (separate components)
        let github_owner = repo.owner();
        let github_repo = repo.name();
        assert_eq!(github_owner, "octocat");
        assert_eq!(github_repo, "Hello-World");

        // Round-trip test
        let parsed = Repository::parse(&db_format).unwrap();
        assert_eq!(parsed.owner(), github_owner);
        assert_eq!(parsed.name(), github_repo);
    }
}
