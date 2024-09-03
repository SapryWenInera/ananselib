use {
    crate::specs::attribute::Attributes,
    std::{
        ffi::OsStr,
        fmt::Debug,
        hash::Hash,
        ops::Deref,
        path::{Component, Path, PathBuf},
    },
};

pub(crate) trait Sanitize {
    fn sanitize(&mut self);
}

macro_rules! update {
    ($metadata:ident, $attribute:ident, $field:ident) => {
        if $metadata.$field != $attribute.$field {
            $metadata.$field = $attribute.$field;
        }
    };
    ($metadata:ident, $attribute:ident, $field:ident.$subfield:ident) => {
        if $metadata.$field.$subfield != $attribute.$field.$subfield {
            $metadata.$field.$subfield = $attribute.$field.$subfield;
        }
    };
}

#[derive(Clone)]
pub struct ZipPath {
    inner: Box<OsStr>,
    pub metadata: Option<Attributes>,
}

impl Debug for ZipPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(metadata) = &self.metadata {
            write!(f, "{:?} {:?}", self.inner, metadata)
        } else {
            write!(f, "{:?}", self.inner)
        }
    }
}

impl Default for ZipPath {
    fn default() -> Self {
        Self {
            inner: OsStr::new("").into(),
            metadata: None,
        }
    }
}

impl Deref for ZipPath {
    type Target = OsStr;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Eq for ZipPath {}

impl Ord for ZipPath {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<T> From<T> for ZipPath
where
    T: AsRef<OsStr>,
{
    fn from(value: T) -> Self {
        let inner = value.as_ref();

        Self {
            inner: inner.into(),
            ..Self::default()
        }
    }
}

impl Hash for ZipPath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl PartialEq for ZipPath {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn ne(&self, other: &Self) -> bool {
        self.inner != other.inner
    }
}

impl PartialOrd for ZipPath {
    fn ge(&self, other: &Self) -> bool {
        self.inner >= other.inner
    }

    fn gt(&self, other: &Self) -> bool {
        self.inner > other.inner
    }

    fn le(&self, other: &Self) -> bool {
        self.inner <= other.inner
    }

    fn lt(&self, other: &Self) -> bool {
        self.inner < other.inner
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl Sanitize for ZipPath {
    fn sanitize(&mut self) {
        let path: PathBuf = Path::new(&self.inner)
            .components()
            .filter(|c| matches!(c, Component::Normal(_)))
            .collect();

        self.inner = path.as_os_str().into();
    }
}

impl ZipPath {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn as_os_str(&self) -> &OsStr {
        OsStr::new(&*self.inner)
    }

    pub fn append<S>(&mut self, string: S)
    where
        S: AsRef<Path>,
    {
        let mut path = PathBuf::from(&self.inner);
        path.extend(string.as_ref());
        self.inner = path.as_os_str().into()
    }

    pub fn is_dir(&self) -> bool {
        match &self.metadata {
            Some(attribute) => attribute.directory,
            None => false,
        }
    }

    pub fn is_file(&self) -> bool {
        match &self.metadata {
            Some(attribute) => attribute.file,
            None => false,
        }
    }

    pub fn is_symlink(&self) -> bool {
        match &self.metadata {
            Some(attribute) => attribute.symbolic,
            None => false,
        }
    }

    pub fn file_name(&self) -> Option<&Path> {
        if self.is_file() {
            let path = Path::new(&self.inner)
                .components()
                .last()
                .map(|c| Path::new(c.as_os_str()));
            path
        } else {
            None
        }
    }

    pub fn parent(&self) -> Option<&Path> {
        Path::new(&self.inner).parent()
    }

    pub fn update(&mut self, attribute: &Attributes) {
        if let Some(metadata) = &mut self.metadata {
            update!(metadata, attribute, directory);
            update!(metadata, attribute, symbolic);
            update!(metadata, attribute, file);
            update!(metadata, attribute, owner.write);
            update!(metadata, attribute, owner.execute);
            update!(metadata, attribute, owner.read);
            update!(metadata, attribute, group.write);
        } else {
            self.metadata = Some(attribute.clone());
        }
    }
}
