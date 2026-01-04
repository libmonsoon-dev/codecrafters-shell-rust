use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::slice::Iter;
use std::sync;

pub struct BinPath {
    env_once: sync::Once,
    path: Vec<String>,
}

impl BinPath {
    pub fn new() -> Self {
        Self {
            env_once: sync::Once::new(),
            path: Vec::new(),
        }
    }

    pub fn lookup(&mut self, bin: &str) -> io::Result<Option<PathBuf>> {
        self.load_path();
        for dir in &self.path {
            let path = Path::new(&dir).join(bin);
            let result = fs::metadata(path.clone());
            if matches!(result, Err(ref err) if err.kind() == io::ErrorKind::NotFound) {
                continue;
            }

            if has_execute_permission(&result?) {
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    pub fn bins(&mut self) -> Bins<'_> {
        self.load_path();

        Bins::new(self.path.iter())
    }

    fn load_path(&mut self) {
        self.env_once.call_once(|| {
            self.path = env::var("PATH")
                .unwrap()
                .split(':')
                .map(String::from)
                .collect();
        })
    }
}

pub struct Bins<'a> {
    paths: Iter<'a, String>,
    dir_data: Option<fs::ReadDir>,
}

impl<'a> Bins<'a> {
    fn new(paths: Iter<'a, String>) -> Self {
        Self {
            paths,
            dir_data: None,
        }
    }
}

impl<'a> Iterator for Bins<'a> {
    type Item = anyhow::Result<PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let dir_data = self.dir_data.take();
            match dir_data {
                Some(mut read_dir) => match read_dir.next() {
                    Some(Ok(dir_entry)) => {
                        self.dir_data = Some(read_dir);

                        match dir_entry.metadata() {
                            Ok(metadata) if has_execute_permission(&metadata) => {
                                return Some(Ok(dir_entry.path()));
                            }
                            Ok(_) => {}
                            Err(err) => {
                                return Some(Err(anyhow::anyhow!(
                                    "read {} metadata: {err}",
                                    dir_entry.path().display()
                                )));
                            }
                        }
                    }
                    Some(Err(err)) => {
                        self.dir_data = Some(read_dir);
                        return Some(Err(anyhow::anyhow!("read dir next: {err}")));
                    }
                    None => self.dir_data = None,
                },
                None => {
                    let Some(dir) = self.paths.next() else {
                        return None;
                    };

                    match fs::read_dir(dir) {
                        Ok(data) => self.dir_data = Some(data),
                        Err(err) => return Some(Err(anyhow::anyhow!("read dir: {err}"))),
                    };
                }
            }
        }
    }
}

//TODO: handle user and group permissions
fn has_execute_permission(attr: &fs::Metadata) -> bool {
    attr.permissions().mode() & 0o001 != 0
}
