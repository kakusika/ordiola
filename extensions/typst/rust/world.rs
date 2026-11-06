use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use chrono::Datelike;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;

static SHARED_FONTS: OnceLock<Arc<FontStore>> = OnceLock::new();
static SHARED_LIBRARY: OnceLock<LazyHash<Library>> = OnceLock::new();

/// Returns a shared, lazily initialized font store containing embedded and system fonts.
pub fn get_font_store() -> Arc<FontStore> {
    SHARED_FONTS
        .get_or_init(|| {
            let mut store = FontStore::new();
            store.extend(typst_kit::fonts::embedded());
            store.extend(typst_kit::fonts::system());
            Arc::new(store)
        })
        .clone()
}

/// Returns a shared, lazily initialized standard library instance.
pub fn get_library() -> &'static LazyHash<Library> {
    SHARED_LIBRARY.get_or_init(|| LazyHash::new(Library::default()))
}

pub struct TypstWorld {
    library: &'static LazyHash<Library>,
    fonts: Arc<FontStore>,
    main_id: FileId,
    main_source: Source,
    vault_root: PathBuf,
    file_cache: Mutex<HashMap<FileId, FileResult<Bytes>>>,
    source_cache: Mutex<HashMap<FileId, FileResult<Source>>>,
    now: chrono::DateTime<chrono::Local>,
}

impl TypstWorld {
    pub fn new(
        source_text: String,
        relative_path: Option<&str>,
        vault_root: Option<&Path>,
    ) -> Self {
        let root = vault_root
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let rel = relative_path.unwrap_or("main.typ");
        let vpath = VirtualPath::new(rel).unwrap_or_else(|_| VirtualPath::new("main.typ").unwrap());
        let rooted = RootedPath::new(VirtualRoot::Project, vpath);
        let main_id = rooted.intern();
        let main_source = Source::new(main_id, source_text);

        Self {
            library: get_library(),
            fonts: get_font_store(),
            main_id,
            main_source,
            vault_root: root,
            file_cache: Mutex::new(HashMap::new()),
            source_cache: Mutex::new(HashMap::new()),
            now: chrono::Local::now(),
        }
    }

    pub fn main_source(&self) -> &Source {
        &self.main_source
    }

    fn resolve_file_id(&self, id: FileId) -> FileResult<PathBuf> {
        let vpath = id.vpath();
        vpath
            .realize(&self.vault_root)
            .map_err(|_| FileError::NotFound(PathBuf::from(vpath.get_with_slash())))
    }
}

impl World for TypstWorld {
    fn library(&self) -> &LazyHash<Library> {
        self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_id {
            return Ok(self.main_source.clone());
        }

        let mut cache = self.source_cache.lock().unwrap();
        if let Some(res) = cache.get(&id) {
            return res.clone();
        }

        let path = self.resolve_file_id(id)?;
        let content = std::fs::read_to_string(&path)
            .map_err(|e| FileError::from_io(e, &path))?;
        let src = Source::new(id, content);
        cache.insert(id, Ok(src.clone()));
        Ok(src)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.main_id {
            return Ok(Bytes::from_string(self.main_source.text().to_string()));
        }

        let mut cache = self.file_cache.lock().unwrap();
        if let Some(res) = cache.get(&id) {
            return res.clone();
        }

        let path = self.resolve_file_id(id)?;
        let data = std::fs::read(&path)
            .map_err(|e| FileError::from_io(e, &path))?;
        let bytes = Bytes::new(data);
        cache.insert(id, Ok(bytes.clone()));
        Ok(bytes)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let mut date = self.now;
        if let Some(offset) = offset {
            let seconds = offset.seconds();
            date += chrono::Duration::seconds(seconds.trunc() as i64);
        }
        Datetime::from_ymd(date.year(), date.month() as u8, date.day() as u8)
    }
}
