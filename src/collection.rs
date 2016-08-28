use std::io::prelude::*;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use toml;

use vfs::*;
use error::*;

#[derive(Debug, RustcEncodable)]
pub struct Song {
    path: String,
    display_name: String,
}

impl Song {
    pub fn read(collection: &Collection, file: &fs::DirEntry) -> Result<Song, PError> {
        let file_meta = try!(file.metadata());
        assert!(file_meta.is_file());

        let file_path = file.path();
        let file_path = file_path.as_path();
        let virtual_path = try!(collection.vfs.real_to_virtual(file_path));
        let path_string = try!(virtual_path.to_str().ok_or(PError::PathDecoding));

        let display_name = virtual_path.file_stem().unwrap();
        let display_name = display_name.to_str().unwrap();
        let display_name = display_name.to_string();

        Ok(Song {
            path: path_string.to_string(),
            display_name: display_name,
        })
    }
}

#[derive(Debug, RustcEncodable)]
pub struct Directory {
    path: String,
    display_name: String,
}

impl Directory {
    pub fn read(collection: &Collection,
                file: &fs::DirEntry)
                -> Result<Directory, PError> {
        let file_meta = try!(file.metadata());
        assert!(file_meta.is_dir());

        let file_path = file.path();
        let file_path = file_path.as_path();
        let virtual_path = try!(collection.vfs.real_to_virtual(file_path));
        let path_string = try!(virtual_path.to_str().ok_or(PError::PathDecoding));

        let display_name = virtual_path.iter().last().unwrap();
        let display_name = display_name.to_str().unwrap();
        let display_name = display_name.to_string();

        Ok(Directory {
            path: path_string.to_string(),
            display_name: display_name,
        })
    }
}

#[derive(Debug, RustcEncodable)]
pub enum CollectionFile {
    Directory(Directory),
    Song(Song),
}

pub struct Collection {
    vfs: Vfs,
}

const CONFIG_MOUNT_DIRS : &'static str = "mount_dirs";
const CONFIG_MOUNT_DIR_NAME : &'static str = "name";
const CONFIG_MOUNT_DIR_SOURCE : &'static str = "source";

impl Collection {
    pub fn new() -> Collection {
        Collection { vfs: Vfs::new() }
    }

    pub fn load_config(&mut self, config_path: &Path) -> Result<(), PError>
    {
        // Open
        let mut config_file = match File::open(config_path) {
            Ok(c) => c,
            Err(_) => return Err(PError::ConfigFileOpenError),
        };

        // Read
        let mut config_file_content = String::new();
        match config_file.read_to_string(&mut config_file_content) {
            Ok(_) => (),
            Err(_) => return Err(PError::ConfigFileReadError),
        };

        // Parse
        let parsed_config = toml::Parser::new(config_file_content.as_str()).parse();
        let parsed_config = match parsed_config {
            Some(c) => c,
            None => return Err(PError::ConfigFileParseError),
        };

        // Apply
        try!(self.load_config_mount_points(&parsed_config));

        Ok(())
    }

    fn load_config_mount_points(&mut self, config: &toml::Table) -> Result<(), PError> {
        let mount_dirs = match config.get(CONFIG_MOUNT_DIRS) {
            Some(s) => s,
            None => return Ok(()),
        };

        let mount_dirs = match mount_dirs {
            &toml::Value::Array(ref a) => a,
            _ => return Err(PError::ConfigMountDirsParseError),
        };

        for dir in mount_dirs {
           let name = match dir.lookup(CONFIG_MOUNT_DIR_NAME) {
               None => return Err(PError::ConfigMountDirsParseError),
               Some(n) => n,
           };
           let name = match name.as_str() {
               None => return Err(PError::ConfigMountDirsParseError),
               Some(n) => n,
           };

           let source = match dir.lookup(CONFIG_MOUNT_DIR_SOURCE) {
               None => return Err(PError::ConfigMountDirsParseError),
               Some(n) => n,
           };
           let source = match source.as_str() {
               None => return Err(PError::ConfigMountDirsParseError),
               Some(n) => n,
           };
           let source = PathBuf::from(source);
           
           try!(self.mount(name, source.as_path()));
        }

        Ok(())
    }

    pub fn mount(&mut self, name: &str, real_path: &Path) -> Result<(), PError> {
        self.vfs.mount(name, real_path)
    }

    pub fn browse(&self, path: &Path) -> Result<Vec<CollectionFile>, PError> {

        let full_path = try!(self.vfs.virtual_to_real(path));

        let mut out = vec![];
        for file in try!(fs::read_dir(full_path)) {
            let file = try!(file);
            let file_meta = try!(file.metadata());
            if file_meta.is_file() {
                let song = try!(Song::read(self, &file));
                out.push(CollectionFile::Song(song));
            } else if file_meta.is_dir() {
                let directory = try!(Directory::read(self, &file));
                out.push(CollectionFile::Directory(directory));
            }
        }

        Ok(out)
    }

    fn flatten_internal(&self, path: &Path) -> Result<Vec<Song>, PError> {
        let files = try!(fs::read_dir(path));
        files.fold(Ok(vec![]), |acc, file| {
            let mut acc = try!(acc);
            let file: fs::DirEntry = try!(file);
            let file_meta = try!(file.metadata());
            if file_meta.is_file() {
                let song = try!(Song::read(self, &file));
                acc.push(song);
            } else {
                let explore_path = file.path();
                let explore_path = explore_path.as_path();
                let mut explore_content = try!(self.flatten_internal(explore_path));
                acc.append(&mut explore_content);
            }
            Ok(acc)
        })
    }

    pub fn flatten(&self, path: &Path) -> Result<Vec<Song>, PError> {
        let real_path = try!(self.vfs.virtual_to_real(path));
        self.flatten_internal(real_path.as_path())
    }

    pub fn locate(&self, virtual_path: &Path) -> Result<PathBuf, PError> {
        self.vfs.virtual_to_real(virtual_path)
    }
}
