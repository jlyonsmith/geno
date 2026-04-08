use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

/// Trait for resolving file paths and reading file contents.
pub trait FileResolver {
    /// Pushes a [PathBuf] onto the resolver's internal stack of parent paths, fully
    /// resolving it to an absolute path in the process.
    fn push_path(&mut self, path: &Path) -> Result<(), ResolverError>;
    /// Pops the last [PathBuf] from the resolver's internal stack of parent paths.
    fn pop_path(&mut self);
    /// Returns the current file path being resolved
    fn current_path(&self) -> Option<&PathBuf>;
    /// Reads the contents of the current file being resolved as a string.
    fn read_to_string(&self) -> Result<String, ResolverError>;
}

/// Error type for file resolver operations.
#[derive(Debug)]
pub enum ResolverError {
    /// A duplicate include path was encountered.
    DuplicateInclude(PathBuf),
    /// An IO error occurred.
    Io(PathBuf, std::io::Error),
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Io(p, e) => write!(f, "resolver i/o failure '{}': {}", p.display(), e),
            Self::DuplicateInclude(p) => write!(f, "Duplicate include: {}", p.display()),
        }
    }
}

impl Error for ResolverError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(_, e) => Some(e), // Return the underlying IO error
            Self::DuplicateInclude(_) => None,
        }
    }
}

/// A standard file resolver that uses a [`HashSet`] to track included files.
pub struct StandardFileResolver {
    files: Vec<PathBuf>,
}

impl StandardFileResolver {
    /// Creates a new [`StandardFileResolver`] with an empty [`HashSet`].
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }
}

impl FileResolver for StandardFileResolver {
    fn push_path(&mut self, path: &Path) -> Result<(), ResolverError> {
        let mut full_path = if self.files.len() == 0 {
            let current_dir =
                std::env::current_dir().map_err(|e| ResolverError::Io(path.to_path_buf(), e))?;

            current_dir.join(path)
        } else {
            self.current_path().unwrap().parent().unwrap().join(path)
        };

        full_path = path_clean::clean(full_path);

        if self.files.iter().find(|path| *path == &full_path).is_none() {
            self.files.push(full_path);
            Ok(())
        } else {
            Err(ResolverError::DuplicateInclude(full_path))
        }
    }

    fn pop_path(&mut self) {
        self.files.pop();
    }

    fn current_path(&self) -> Option<&PathBuf> {
        self.files.last()
    }

    fn read_to_string(&self) -> Result<String, ResolverError> {
        let file_path = self.current_path().unwrap();

        match std::fs::read_to_string(file_path) {
            Ok(contents) => Ok(contents),
            Err(e) => Err(ResolverError::Io(file_path.to_path_buf(), e)),
        }
    }
}
