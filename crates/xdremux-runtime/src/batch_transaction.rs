use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::batch_checkpoint::{
    canonical_or_absolute, source_signature_path, validate_checkpoint_path, SourceSignature,
};
use crate::live_photo_package::{
    planned_members, read_members, rename_exclusive, validate_output_parent,
};
use crate::{publication_parent, Result, RuntimeError};

const STAGE_PREFIX: &str = ".xdremux-batch-stage-";
pub const SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MotionLayout {
    File,
    Pair,
    Package,
}

impl MotionLayout {
    fn members(self, output: &Path) -> Result<Vec<PathBuf>> {
        Ok(match self {
            Self::File => vec![output.to_owned()],
            // Publish the companion first, as in the single-file Live Photo route.
            Self::Pair => vec![output.with_extension("mov"), output.to_owned()],
            Self::Package => {
                let members = planned_members(output)?;
                vec![members.image, members.video, output.join("metadata.plist")]
            }
        })
    }

    fn verify_package(self, path: &Path) -> Result<()> {
        if self == Self::Package {
            read_members(path)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct TransactionKey {
    source: PathBuf,
    source_size: u64,
    source_sha256: String,
    route: String,
    parent: PathBuf,
}

impl TransactionKey {
    pub(crate) fn new(
        source: &Path,
        signature: &SourceSignature,
        route: String,
        parent: &Path,
    ) -> Result<Self> {
        Ok(Self {
            source: canonical_or_absolute(source)?,
            source_size: signature.size,
            source_sha256: signature.sha256.clone(),
            route,
            parent: canonical_or_absolute(parent)?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Publication {
    pub(crate) output: PathBuf,
    layout: MotionLayout,
    stage: String,
    sha256: Vec<String>,
    committed: bool,
}

enum Destination {
    Missing,
    Partial(Vec<PathBuf>),
    Complete,
}

impl Publication {
    pub(crate) fn prepared(output: &Path, layout: MotionLayout, stage: &Stage) -> Result<Self> {
        layout.verify_package(&stage.output)?;
        let sha256 = layout
            .members(&stage.output)?
            .iter()
            .map(|member| Ok(source_signature_path(member)?.sha256))
            .collect::<Result<Vec<_>>>()?;
        let output = destination_path(output)?;
        Ok(Self {
            output,
            layout,
            stage: stage.nonce.clone(),
            sha256,
            committed: false,
        })
    }

    pub(crate) fn stage_directory(&self) -> PathBuf {
        publication_parent(&self.output).join(format!("{STAGE_PREFIX}{}", self.stage))
    }

    pub(crate) fn outputs(&self) -> Vec<PathBuf> {
        match self.layout {
            MotionLayout::Pair => vec![self.output.clone(), self.output.with_extension("mov")],
            _ => vec![self.output.clone()],
        }
    }

    fn destination(&self) -> Result<Destination> {
        let paths = self.layout.members(&self.output)?;
        if self.layout == MotionLayout::Package && entry_exists(&self.output)? {
            self.layout.verify_package(&self.output)?;
        }
        let mut matched = Vec::new();
        for (member, expected) in paths.iter().zip(&self.sha256) {
            if entry_exists(member)? {
                require_digest(member, expected)?;
                matched.push(member.clone());
            }
        }
        if matched.len() == paths.len() {
            Ok(Destination::Complete)
        } else if matched.is_empty() {
            Ok(Destination::Missing)
        } else {
            Ok(Destination::Partial(matched))
        }
    }

    /// Return None only when the whole destination is missing and can be rebuilt.
    /// An owned, verified stage can complete a partial pair without clobbering.
    pub(crate) fn publish(
        &self,
        expected_layout: MotionLayout,
        observed: &mut Vec<PathBuf>,
    ) -> Result<bool> {
        if self.layout != expected_layout {
            return Err(problem("journal layout does not match the current request"));
        }
        validate_output_parent(&self.output)?;
        let present = match self.destination()? {
            Destination::Complete => {
                *observed = self.outputs();
                return Ok(true);
            }
            Destination::Missing => Vec::new(),
            Destination::Partial(paths) if !self.committed => paths,
            Destination::Partial(_) => {
                return Err(problem(
                    "a committed Live Photo is missing a member; preserving all entries",
                ))
            }
        };
        let stage = self.stage_directory();
        let stage_output = stage.join(
            self.output
                .file_name()
                .expect("validated journal output name"),
        );
        let valid_stage = !self.committed
            && fs::symlink_metadata(&stage).is_ok_and(|value| value.is_dir())
            && entry_exists(&stage_output)?;
        if !valid_stage {
            if present.is_empty() {
                return Ok(false);
            }
            observed.clone_from(&present);
            return Err(problem(format!(
                "partial output needs its verified staging directory {}",
                stage.display()
            )));
        }
        self.layout.verify_package(&stage_output)?;
        let staged = self.layout.members(&stage_output)?;
        for (member, expected) in staged.iter().zip(&self.sha256) {
            require_digest(member, expected)?;
        }
        observed.clone_from(&present);
        match self.layout {
            MotionLayout::Package => {
                rename_exclusive(&stage_output, &self.output).map_err(io_error)?;
                observed.push(self.output.clone());
            }
            MotionLayout::File => {
                // Keep the platform's existing single-file no-clobber
                // primitive. The stage is still available if it loses a race.
                let mut temporary =
                    tempfile::TempPath::try_from_path(stage_output).map_err(io_error)?;
                temporary.disable_cleanup(true);
                temporary
                    .persist_noclobber(&self.output)
                    .map_err(|error| io_error(error.error))?;
                observed.push(self.output.clone());
            }
            MotionLayout::Pair => {
                for (source, final_path) in staged.iter().zip(self.layout.members(&self.output)?) {
                    if !present.contains(&final_path) {
                        fs::hard_link(source, &final_path).map_err(io_error)?;
                        observed.push(final_path);
                    }
                }
            }
        }
        if !matches!(self.destination()?, Destination::Complete) {
            return Err(problem("published media failed readback"));
        }
        *observed = self.outputs();
        Ok(true)
    }

    pub(crate) fn committed(mut self) -> Self {
        self.committed = true;
        self
    }

    pub(crate) fn is_committed(&self) -> bool {
        self.committed
    }

    pub(crate) fn remove_staging(&self) {
        let stage = self.stage_directory();
        if !fs::symlink_metadata(&stage).is_ok_and(|value| value.is_dir()) {
            return;
        }
        let Some(name) = self.output.file_name() else {
            return;
        };
        let staged = stage.join(name);
        let matches = self.layout.verify_package(&staged).is_ok()
            && self.layout.members(&staged).is_ok_and(|paths| {
                paths
                    .iter()
                    .zip(&self.sha256)
                    .all(|(path, digest)| require_digest(path, digest).is_ok())
            });
        let expected_entries = if self.layout == MotionLayout::Pair {
            vec![staged.clone(), staged.with_extension("mov")]
        } else {
            vec![staged]
        };
        let entries = fs::read_dir(&stage).and_then(|entries| {
            entries
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<io::Result<Vec<_>>>()
        });
        if matches
            && entries.is_ok_and(|entries| {
                entries.len() == expected_entries.len()
                    && entries.iter().all(|entry| expected_entries.contains(entry))
            })
        {
            let _ = fs::remove_dir_all(stage);
        } else {
            // The single file or package may have moved out of the stage.
            let _ = fs::remove_dir(stage);
        }
    }
}

pub(crate) struct Stage {
    temporary: tempfile::TempDir,
    nonce: String,
    pub(crate) output: PathBuf,
}

impl Stage {
    pub(crate) fn new(output: &Path) -> Result<Self> {
        let nonce = Uuid::new_v4().simple().to_string();
        let temporary = tempfile::Builder::new()
            .prefix(&format!("{STAGE_PREFIX}{nonce}"))
            .rand_bytes(0)
            .tempdir_in(publication_parent(output))
            .map_err(io_error)?;
        let name = output
            .file_name()
            .ok_or_else(|| problem("missing batch output file name"))?;
        let output = temporary.path().join(name);
        Ok(Self {
            temporary,
            nonce,
            output,
        })
    }

    pub(crate) fn retain(self) {
        let _ = self.temporary.keep();
    }
}

#[derive(Default)]
pub(crate) struct TransactionIndex(BTreeMap<TransactionKey, Publication>);

impl TransactionIndex {
    pub(crate) fn load(logical: &Path) -> Result<Self> {
        let path = journal_path(logical);
        match File::open(path) {
            Ok(file) => Ok(read_journal(&file)?.0),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(io_error(error)),
        }
    }

    pub(crate) fn get(&self, key: &TransactionKey) -> Option<&Publication> {
        self.0.get(key)
    }

    pub(crate) fn reserves(&self, destination: &Path) -> bool {
        self.0
            .values()
            .any(|entry| !entry.committed && entry.output == destination)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum JournalLine {
    Header {
        schema_version: u32,
    },
    Publication {
        key: TransactionKey,
        publication: Publication,
    },
}

pub(crate) fn destination_path(output: &Path) -> Result<PathBuf> {
    let name = output
        .file_name()
        .ok_or_else(|| problem("missing batch output file name"))?;
    Ok(canonical_or_absolute(publication_parent(output))?.join(name))
}

pub(crate) fn journal_path(logical: &Path) -> PathBuf {
    let mut path = logical.as_os_str().to_owned();
    path.push(".v4");
    PathBuf::from(path)
}

fn read_journal(mut file: &File) -> Result<(TransactionIndex, u64)> {
    file.rewind().map_err(io_error)?;
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::new();
    let mut index = TransactionIndex::default();
    let mut complete = 0;
    loop {
        bytes.clear();
        if reader.read_until(b'\n', &mut bytes).map_err(io_error)? == 0
            || bytes.last() != Some(&b'\n')
        {
            return Ok((index, complete));
        }
        let line: JournalLine = serde_json::from_slice(&bytes)
            .map_err(|error| RuntimeError::external("Motion Photo checkpoint record", error))?;
        match line {
            JournalLine::Header {
                schema_version: SCHEMA_VERSION,
            } if complete == 0 => {}
            JournalLine::Publication { key, publication } if complete > 0 => {
                if publication.output.parent() != Some(key.parent.as_path())
                    || publication.output.file_name().is_none()
                    || !Uuid::parse_str(&publication.stage)
                        .is_ok_and(|nonce| nonce.simple().to_string() == publication.stage)
                    || publication.layout.members(&publication.output)?.len()
                        != publication.sha256.len()
                {
                    return Err(problem("invalid transaction identity in checkpoint"));
                }
                index.0.insert(key, publication);
            }
            _ => return Err(problem("invalid v4 checkpoint header")),
        }
        complete += bytes.len() as u64;
    }
}

pub(crate) struct BatchJournal {
    path: PathBuf,
    protected: Vec<PathBuf>,
    session: Mutex<Option<OpenJournal>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JournalHealth {
    Healthy,
    Poisoned,
}

struct OpenJournal {
    file: File,
    index: TransactionIndex,
    health: JournalHealth,
}

impl BatchJournal {
    pub(crate) fn new(logical: &Path, protected: Vec<PathBuf>) -> Result<Self> {
        let path = destination_path(&journal_path(logical))?;
        validate_checkpoint_path(logical, protected.iter().map(PathBuf::as_path))?;
        validate_checkpoint_path(&path, protected.iter().map(PathBuf::as_path))?;
        Ok(Self {
            path,
            protected,
            session: Mutex::new(None),
        })
    }

    fn session(&self) -> Result<MutexGuard<'_, Option<OpenJournal>>> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| problem("checkpoint writer lock poisoned"))?;
        if session.is_none() {
            validate_output_parent(&self.path)?;
            fs::create_dir_all(publication_parent(&self.path)).map_err(io_error)?;
            // Open without truncation and compare the opened descriptor with
            // the protected paths before acquiring a write lease.
            validate_checkpoint_path(&self.path, self.protected.iter().map(PathBuf::as_path))?;
            let mut options = OpenOptions::new();
            options.read(true);
            // Windows append access omits FILE_WRITE_DATA, which set_len
            // needs to discard a torn tail. write_line seeks under the lock.
            #[cfg(windows)]
            options.write(true);
            #[cfg(not(windows))]
            options.append(true);
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32);
            }
            let mut file = match options.clone().create_new(true).open(&self.path) {
                Ok(file) => file,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    options.open(&self.path).map_err(io_error)?
                }
                Err(error) => return Err(io_error(error)),
            };
            let handle = same_file::Handle::from_file(file.try_clone().map_err(io_error)?)
                .map_err(io_error)?;
            for path in &self.protected {
                if path.is_file()
                    && same_file::Handle::from_path(path).is_ok_and(|other| handle == other)
                {
                    return Err(problem(format!(
                        "checkpoint aliases protected file {}",
                        path.display()
                    )));
                }
            }
            if !file.metadata().map_err(io_error)?.is_file() {
                return Err(problem("checkpoint is not a regular file"));
            }
            file.try_lock()
                .map_err(|error| RuntimeError::external("Motion Photo checkpoint lock", error))?;
            let (index, complete) = read_journal(&file)?;
            if file.metadata().map_err(io_error)?.len() != complete {
                file.set_len(complete).map_err(io_error)?;
                file.sync_all().map_err(io_error)?;
            }
            if complete == 0 {
                write_line(
                    &mut file,
                    &JournalLine::Header {
                        schema_version: SCHEMA_VERSION,
                    },
                )?;
            }
            *session = Some(OpenJournal {
                file,
                index,
                health: JournalHealth::Healthy,
            });
        }
        Ok(session)
    }

    pub(crate) fn get(&self, key: &TransactionKey) -> Result<Option<Publication>> {
        let session = self.session()?;
        Ok(session
            .as_ref()
            .expect("open journal")
            .index
            .get(key)
            .cloned())
    }

    pub(crate) fn append(&self, key: &TransactionKey, publication: &Publication) -> Result<()> {
        let mut session = self.session()?;
        let state = session.as_mut().expect("open journal");
        if state.health == JournalHealth::Poisoned {
            return Err(problem(
                "previous checkpoint write failed; resume with a new invocation",
            ));
        }
        let result = write_line(
            &mut state.file,
            &JournalLine::Publication {
                key: key.clone(),
                publication: publication.clone(),
            },
        );
        if result.is_err() {
            state.health = JournalHealth::Poisoned;
            return result;
        }
        state.index.0.insert(key.clone(), publication.clone());
        Ok(())
    }
}

fn write_line(file: &mut File, record: &JournalLine) -> Result<()> {
    let mut bytes = serde_json::to_vec(record)
        .map_err(|error| RuntimeError::external("Motion Photo checkpoint encoding", error))?;
    bytes.push(b'\n');
    file.seek(SeekFrom::End(0)).map_err(io_error)?;
    file.write_all(&bytes).map_err(io_error)?;
    file.sync_all().map_err(io_error)
}

fn entry_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error(error)),
    }
}

fn require_digest(path: &Path, expected: &str) -> Result<()> {
    if !fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
        || source_signature_path(path)?.sha256 != expected
    {
        return Err(problem(format!(
            "existing media differs from checkpoint provenance; preserving {}",
            path.display()
        )));
    }
    Ok(())
}

fn problem(detail: impl Into<String>) -> RuntimeError {
    RuntimeError::new("Motion Photo transaction", detail)
}

fn io_error(error: io::Error) -> RuntimeError {
    RuntimeError::external("Motion Photo transaction", error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_pair_completes_from_prepared_member_identities() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("input.jpg");
        let output = root.path().join("out.heic");
        let logical = root.path().join("checkpoint");
        fs::write(&source, b"original").unwrap();
        let signature = source_signature_path(&source).unwrap();
        let key =
            TransactionKey::new(&source, &signature, "apple-pair".to_owned(), root.path()).unwrap();
        let stage = Stage::new(&output).unwrap();
        fs::write(&stage.output, b"verified still").unwrap();
        fs::write(stage.output.with_extension("mov"), b"verified movie").unwrap();

        let prepared = Publication::prepared(&output, MotionLayout::Pair, &stage).unwrap();
        let journal = BatchJournal::new(&logical, vec![source.clone(), output.clone()]).unwrap();
        journal.append(&key, &prepared).unwrap();
        let movie = output.with_extension("mov");
        fs::hard_link(stage.output.with_extension("mov"), &movie).unwrap();
        stage.retain();
        drop(journal);

        let resumed = BatchJournal::new(&logical, vec![source.clone(), output.clone()]).unwrap();
        let record = resumed.get(&key).unwrap().unwrap();
        let mut outputs = Vec::new();
        assert!(!record.is_committed());
        assert!(record.publish(MotionLayout::Pair, &mut outputs).unwrap());
        assert_eq!(
            outputs,
            vec![
                destination_path(&output).unwrap(),
                destination_path(&movie).unwrap()
            ]
        );
        assert_eq!(fs::read(&output).unwrap(), b"verified still");
        assert_eq!(fs::read(&movie).unwrap(), b"verified movie");
        let committed = record.committed();
        resumed.append(&key, &committed).unwrap();
        committed.remove_staging();
        assert!(!committed.stage_directory().exists());
        assert_eq!(fs::read(&source).unwrap(), b"original");
    }

    #[test]
    fn competing_writer_cannot_modify_locked_checkpoint() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("input");
        let logical = root.path().join("checkpoint");
        fs::write(&source, b"original").unwrap();
        let key = TransactionKey::new(
            &source,
            &source_signature_path(&source).unwrap(),
            "apple-pair".to_owned(),
            root.path(),
        )
        .unwrap();
        let first = BatchJournal::new(&logical, vec![source.clone()]).unwrap();
        assert!(first.get(&key).unwrap().is_none());
        let other = BatchJournal::new(&logical, vec![source]).unwrap();
        assert!(other
            .get(&key)
            .unwrap_err()
            .to_string()
            .contains("checkpoint lock"));
        drop(first);
        assert!(other.get(&key).unwrap().is_none());
    }
}
