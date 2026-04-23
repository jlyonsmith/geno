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
#[derive(Debug, Clone)]
pub enum ResolverError {
    /// A duplicate include path was encountered.
    DuplicateInclude(PathBuf),
    /// An IO error occurred.
    Io(PathBuf, String),
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Io(p, e) => write!(f, "i/o failure '{}': {}", p.display(), e),
            Self::DuplicateInclude(p) => write!(f, "duplicate include '{}'", p.display()),
        }
    }
}

impl Error for ResolverError {}

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
        let mut full_path = if path.is_relative() {
            // If this is the first path, resolve it relative to the current directory
            if self.files.len() == 0 {
                let current_dir = std::env::current_dir()
                    .map_err(|e| ResolverError::Io(path.to_path_buf(), e.to_string()))?;

                current_dir.join(path)
            } else {
                self.current_path().unwrap().parent().unwrap().join(path)
            }
        } else {
            path.to_path_buf()
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
            Err(e) => Err(ResolverError::Io(file_path.to_path_buf(), e.to_string())),
        }
    }
}
