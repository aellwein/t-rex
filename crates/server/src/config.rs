//! Runtime configuration read from environment variables.

use std::{env, path::PathBuf};

use url::Url;

/// Default SQLite location unless `T_REX_DB_PATH` names one. Pure, so the rules can be tested.
pub fn db_path_from(override_path: Option<String>, home: Option<PathBuf>) -> PathBuf {
    let override_path = override_path
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty());
    match override_path {
        Some(path) => PathBuf::from(path),
        None => home
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".t-rex")
            .join("t-rex.db"),
    }
}

/// Which database the server uses.
pub enum DatabaseConfig {
    /// A SQLite file. The default when `DATABASE_URL` is not set.
    Sqlite(PathBuf),
    /// A Postgres server. The URL never contains the password; it is added by [`DatabaseConfig::connection_url`].
    Postgres { url: Url, password: Option<String> },
}

impl DatabaseConfig {
    /// The connection string for the database driver, including the password if there is one.
    pub fn connection_url(&self) -> String {
        match self {
            DatabaseConfig::Sqlite(path) => format!("sqlite://{}?mode=rwc", path.display()),
            DatabaseConfig::Postgres { url, password } => {
                let mut full = url.clone();
                if let Some(password) = password {
                    // Cannot fail: the URL has a host, which is checked when the config is built.
                    let _ = full.set_password(Some(password));
                }
                full.to_string()
            }
        }
    }

    /// The location without any password, for the log.
    pub fn describe(&self) -> String {
        match self {
            DatabaseConfig::Sqlite(path) => path.display().to_string(),
            DatabaseConfig::Postgres { url, .. } => url.to_string(),
        }
    }
}

/// Reads the database settings from the environment. See [`database_config_from`].
pub fn database_config() -> Result<DatabaseConfig, String> {
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    database_config_from(
        env::var("DATABASE_URL").ok(),
        env::var("DATABASE_PASSWORD").ok(),
        env::var("T_REX_DB_PATH").ok(),
        home,
    )
}

/// Pure version of [`database_config`].
///
/// - No `DATABASE_URL`: SQLite file from `T_REX_DB_PATH` (or the default path).
/// - `sqlite://PATH`: SQLite file at `PATH`.
/// - `postgres://USER@HOST:PORT/DB`: Postgres. The password comes from `DATABASE_PASSWORD`,
///   which is mounted from a secret. A password inside the URL is refused, so it stays out of the config.
pub fn database_config_from(
    url: Option<String>,
    password: Option<String>,
    db_path: Option<String>,
    home: Option<PathBuf>,
) -> Result<DatabaseConfig, String> {
    let Some(url) = url.map(|u| u.trim().to_string()).filter(|u| !u.is_empty()) else {
        return Ok(DatabaseConfig::Sqlite(db_path_from(db_path, home)));
    };
    if let Some(path) = url.strip_prefix("sqlite://") {
        return Ok(DatabaseConfig::Sqlite(PathBuf::from(path)));
    }
    let parsed = Url::parse(&url).map_err(|_| "DATABASE_URL is not a valid URL".to_string())?;
    match parsed.scheme() {
        "postgres" | "postgresql" => {
            if parsed.password().is_some() {
                return Err(
                    "put the database password in DATABASE_PASSWORD, not in DATABASE_URL".into(),
                );
            }
            if parsed.host_str().is_none() {
                return Err("DATABASE_URL needs a host".into());
            }
            Ok(DatabaseConfig::Postgres {
                url: parsed,
                password: password.filter(|p| !p.is_empty()),
            })
        }
        other => Err(format!(
            "unsupported database scheme '{other}' in DATABASE_URL"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn default_path_is_inside_the_home_directory() {
        assert_eq!(
            db_path_from(None, Some(PathBuf::from("/home/alex"))),
            PathBuf::from("/home/alex/.t-rex/t-rex.db")
        );
    }

    #[test]
    fn override_wins_over_home_directory() {
        assert_eq!(
            db_path_from(
                Some("/data/t-rex.db".into()),
                Some(PathBuf::from("/home/alex"))
            ),
            PathBuf::from("/data/t-rex.db")
        );
    }

    #[test]
    fn blank_override_is_ignored() {
        assert_eq!(
            db_path_from(Some("   ".into()), Some(PathBuf::from("/home/alex"))),
            PathBuf::from("/home/alex/.t-rex/t-rex.db")
        );
    }

    #[test]
    fn missing_home_falls_back_to_working_directory() {
        assert!(db_path_from(None, None).ends_with(".t-rex/t-rex.db"));
    }

    #[test]
    fn no_url_means_the_sqlite_file() {
        let config = database_config_from(None, None, Some("/data/t-rex.db".into()), None).unwrap();
        assert!(
            matches!(config, DatabaseConfig::Sqlite(path) if path.as_path() == Path::new("/data/t-rex.db"))
        );
    }

    #[test]
    fn sqlite_url_names_the_file() {
        let config =
            database_config_from(Some("sqlite:///tmp/x.db".into()), None, None, None).unwrap();
        assert!(
            matches!(config, DatabaseConfig::Sqlite(path) if path.as_path() == Path::new("/tmp/x.db"))
        );
    }

    #[test]
    fn postgres_password_comes_from_its_own_setting_and_is_escaped() {
        let config = database_config_from(
            Some("postgres://t_rex@db.internal:5432/t_rex".into()),
            Some("p@ss/word".into()),
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            config.connection_url(),
            "postgres://t_rex:p%40ss%2Fword@db.internal:5432/t_rex"
        );
        assert_eq!(config.describe(), "postgres://t_rex@db.internal:5432/t_rex");
    }

    #[test]
    fn postgres_without_password_connects_without_one() {
        let config = database_config_from(
            Some("postgres://t_rex@db.internal/t_rex".into()),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            config.connection_url(),
            "postgres://t_rex@db.internal/t_rex"
        );
    }

    #[test]
    fn password_inside_the_url_is_refused() {
        let err = database_config_from(
            Some("postgres://t_rex:secret@db.internal/t_rex".into()),
            None,
            None,
            None,
        )
        .err()
        .unwrap();
        assert!(err.contains("DATABASE_PASSWORD"));
    }

    #[test]
    fn unknown_scheme_is_refused() {
        assert!(database_config_from(Some("mysql://x@y/z".into()), None, None, None).is_err());
    }
}
