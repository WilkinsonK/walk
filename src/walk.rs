use std::{fs, io::{self, Read}, path};

// TODO: consolidate macro rules.
/// The macro_rules you'll see here are creating
/// predefined predicate callbacks using the API
/// defined in this module.

#[allow(unused_macros)]
#[macro_export]
macro_rules! file_excludes {
    ($pattern:expr) => {
        $crate::Predicate::File(Box::new(|p| {
            let re = regex::Regex::new($pattern).expect("valid regex");
            p.is_dir() || !$crate::file_contains(p, &re)
        }))
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! file_includes {
    ($pattern:expr) => {
        $crate::Predicate::File(Box::new(|p| {
            let re = regex::Regex::new($pattern).expect("valid regex");
            p.is_dir() || $crate::file_contains(p, &re)
        }))
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! file_excludes_format {
    ($kind:expr) => {
        $crate::Predicate::File(Box::new(|p| p.is_dir() || !$crate::file_is_format(p, $kind)))
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! file_includes_format {
    ($kind:expr) => {
        $crate::Predicate::File(|p| p.is_dir() || $crate::file_is_format(p, $kind))
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! parent_excludes {
    ($pattern:expr) => {
        $crate::Predicate::DirHard(Box::new(|p| {
            let re = regex::Regex::new($pattern).expect("valid regex");
            !$crate::parent_contains(p, &re)
        }))
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! parent_includes {
    ($pattern:expr) => {
        $crate::Predicate::DirSoft(Box::new(|p| {
            let re = regex::Regex::new($pattern).expect("valid regex");
            $crate::parent_contains(p, &re)
        }))
    };
}

/// A callable function wrapped by some
/// enumeration of this type. `Predicate`s are
/// inteneded more as a way to homogenize the
/// usage of callbacks that are used for
/// validation, but also allow differentiation
/// between validators that might have a higher
/// precedent than another.
pub enum Predicate {
    // TODO: define generic predictate type.

    /// Soft directory rules are meant for
    /// validating something that might not be
    /// a global issue. That is to say, any
    /// predicate that is a soft-directory rule
    /// might only be applicable once we know the
    /// full path of the child file.
    DirSoft(Box<dyn Fn(&path::Path) -> bool>),
    /// Hard directory rules apply at a higher
    /// scope than soft rules. This indicates that
    /// a violation of this predicate might close
    /// off continued path finding once an FS path
    /// violates this rule.
    DirHard(Box<dyn Fn(&path::Path) -> bool>),
    /// Applies to validation on files at the end
    /// of a path search.
    File(Box<dyn Fn(&path::Path) -> bool>),
    #[allow(dead_code)]
    None,
}

impl Predicate {
    /// Call the wrapped predicate.
    pub fn call(&self, pth: &path::Path) -> bool {
        (match self {
            Self::DirHard(func) => func,
            Self::DirSoft(func) => func,
            Self::File(func)    => func,
            Self::None => panic!("attempted to call on Predicate of None")
        })(pth)
    }

    /// Predicate is intended for directory
    /// part validation.
    #[allow(dead_code)]
    pub fn is_dir(&self) -> bool {
        self.is_dir_hard() || self.is_dir_soft()
    }

    /// Predictate is a hard-rule intended for
    /// directory validation.
    pub fn is_dir_hard(&self) -> bool {
        matches!(self, Self::DirHard(..))
    }

    /// Predicate is a soft-rule intended for
    /// directory validation.
    pub fn is_dir_soft(&self) -> bool {
        matches!(self, Self::DirSoft(..))
    }

    /// Predictate is intended for validating
    /// files.
    #[allow(dead_code)]
    pub fn is_file(&self) -> bool {
        matches!(self, Self::File(..))
    }
}

/// Walks through a file system as configured by
/// properties set, and performs actions on files
/// as dictated by designated callbacks.
pub struct FileWalker<P, C>
where
    P: AsRef<path::Path> + From<path::PathBuf>,
    C: Fn(&path::Path),
{
    location:   P,
    max_depth:  Option<usize>,
    min_depth:  Option<usize>,
    callbacks:  Vec<C>,
    predicates: Vec<Predicate>,
}

impl<P, C> FileWalker<P, C>
where
    P: AsRef<path::Path> + From<path::PathBuf>,
    C: Fn(&path::Path),
{
    /// Create a new instance of a `FileWalker`.
    pub fn new(location: P) -> Self {
        Self{
            location,
            max_depth:  None,
            min_depth:  None,
            callbacks:  vec![],
            predicates: vec![]
        }
    }

    fn add_callback(&mut self, callback: C) {
        self.callbacks.push(callback);
    }

    fn add_predicate(&mut self, predicate: Predicate) {
        self.predicates.push(predicate);
    }

    fn call_callbacks(&self, path: &path::Path) {
        for cb in &self.callbacks {
            cb(path);
        }
    }

    #[allow(dead_code)]
    fn call_pred_dir(&self, pth: &path::Path) -> bool {
        self.call_pred_dirh(pth) && self.call_pred_dirs(pth)
    }

    fn call_pred_dirh(&self, pth: &path::Path) -> bool {
        self.call_predicates(pth, |pr| pr.is_dir_hard())
    }

    fn call_pred_dirs(&self, pth: &path::Path) -> bool {
        self.call_predicates(pth, |pr| pr.is_dir_soft())
    }

    fn call_pred_file(&self, pth: &path::Path) -> bool {
        self.call_predicates(pth, |pr| pr.is_file())
    }

    fn call_predicates(&self, pth: &path::Path, filter: fn(&&Predicate) -> bool) -> bool {
        let validators: Vec<_> = self
            .predicates
            .iter()
            .filter(filter)
            .collect();

        let validations: usize = validators
            .iter()
            .map(|pr| pr.call(pth) as usize)
            .sum();

        validations == validators.len()
    }

    fn set_max_depth(&mut self, depth: usize) {
        self.max_depth = depth.into();
    }

    fn set_min_depth(&mut self, depth: usize) {
        self.min_depth = depth.into();
    }

    /// Walks through the filesystem, using
    /// predicate validations, and performs
    /// tasks from the callback stack in order of
    /// declaration.
    pub fn walk(&self) -> io::Result<()> {
        self.walk_at(&self.location, 0)
    }

    fn walk_at(&self, location: &P, depth: usize) -> io::Result<()> {
        let max_depth = self.max_depth.unwrap_or(usize::MAX);
        let min_depth = self.min_depth.unwrap_or(usize::MIN);

        if depth < max_depth {

            for path in fs::read_dir(location)? {
                let path = path?.path();
                if !self.call_pred_dirh(&path) {
                    break;
                }
                if path.is_dir() {
                    let loc = &path.clone().into();
                    self.walk_at(loc, depth + 1)?;
                    continue;
                }
                if depth < min_depth {
                    continue;
                }

                let dirs_valid = self.call_pred_dirs(&path);
                let file_valid = self.call_pred_file(&path);
                if dirs_valid && file_valid {
                    self.call_callbacks(&path);
                }
            }
        }

        Ok(())
    }

    /// Set the maximum search depth.
    #[allow(dead_code)]
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.set_max_depth(depth);
        self
    }

    /// Set the minimum search depth.
    #[allow(dead_code)]
    pub fn with_min_depth(mut self, depth: usize) -> Self {
        self.set_min_depth(depth);
        self
    }

    /// Adds a callback to the file processor
    /// stack.
    #[allow(dead_code)]
    pub fn with_callback(mut self, cb: C) -> Self {
        self.add_callback(cb);
        self
    }

    /// Adds a predicate callback to the
    /// validation stack.
    #[allow(dead_code)]
    pub fn with_predicate(mut self, cb: Predicate) -> Self {
        self.add_predicate(cb);
        self
    }
}

/// File name matches some pattern.
#[allow(dead_code)]
pub fn file_contains(pth: &path::Path, pattern: &regex::Regex) -> bool {
    let ret = if !pth.is_dir() {
        pattern
            .is_match(pth
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap())
    } else {
        false
    };
    ret
}

/// Path points to some file which is of a
/// specific format.
#[allow(dead_code)]
pub fn file_is_format(pth: &path::Path, kind: &str) -> bool {
    if !pth.is_dir() {
        let mut buf  = [0u8; 256];
        let mut file = fs::File::open(pth).unwrap();
        file.read_exact(&mut buf).unwrap_or(());

        let fmt = file_format::FileFormat::from_bytes(buf);
        fmt.media_type() == kind
    } else {
        false
    }
}

/// Parent path components contains some pattern.
#[allow(dead_code)]
pub fn parent_contains(pth: &path::Path, pattern: &regex::Regex) -> bool {
    pth
        .parent()
        .unwrap()
        .components()
        .any(|c| pattern.is_match(c.as_os_str().to_str().unwrap()))
}
