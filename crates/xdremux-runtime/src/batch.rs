use std::fs;
use std::path::{Path, PathBuf};

use xdremux_engine::ConversionRequest;
use xdremux_source::{probe_bytes, SourceAsset};

use crate::{PortableRuntime, Result, RuntimeError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchItem {
    pub input: PathBuf,
    pub output: PathBuf,
}

impl BatchItem {
    pub fn new(input: impl Into<PathBuf>, output: impl Into<PathBuf>) -> Self {
        Self {
            input: input.into(),
            output: output.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchAssetKind {
    ProXdr,
    LivePhoto,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchSuccess {
    pub input: PathBuf,
    pub outputs: Vec<PathBuf>,
    pub kind: BatchAssetKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchFailure {
    pub input: PathBuf,
    pub output: PathBuf,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BatchReceipt {
    pub successes: Vec<BatchSuccess>,
    pub failures: Vec<BatchFailure>,
}

impl BatchReceipt {
    pub fn processed(&self) -> usize {
        self.successes.len() + self.failures.len()
    }

    pub fn succeeded(&self) -> usize {
        self.successes.len()
    }

    pub fn failed(&self) -> usize {
        self.failures.len()
    }

    pub fn is_success(&self) -> bool {
        self.failures.is_empty()
    }
}

fn create_output_parent(path: &Path) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent)
        .map_err(|error| RuntimeError::external("batch output directory", error))
}

impl PortableRuntime {
    fn convert_batch_item(
        &self,
        item: &BatchItem,
        request: ConversionRequest,
    ) -> Result<BatchSuccess> {
        if item.input == item.output {
            return Err(RuntimeError::new(
                "batch output",
                "batch conversion never overwrites its source",
            ));
        }
        if item.output.exists() {
            return Err(RuntimeError::new(
                "batch output",
                "output already exists; refusing to overwrite",
            ));
        }
        create_output_parent(&item.output)?;
        let source = fs::read(&item.input)
            .map_err(|error| RuntimeError::external("batch input read", error))?;
        let asset = probe_bytes(&source)
            .map_err(|error| RuntimeError::external("batch source probe", error))?;

        match asset {
            SourceAsset::MotionPhoto { .. } => {
                let receipt = self.convert_motion_photo_file(&source, &item.input, &item.output)?;
                Ok(BatchSuccess {
                    input: item.input.clone(),
                    outputs: vec![receipt.image, receipt.video],
                    kind: BatchAssetKind::LivePhoto,
                })
            }
            SourceAsset::ProXdr { .. } => {
                let receipt = self.convert_proxdr_file(&source, &item.output, request, |_| {})?;
                Ok(BatchSuccess {
                    input: item.input.clone(),
                    outputs: vec![receipt.output],
                    kind: BatchAssetKind::ProXdr,
                })
            }
        }
    }

    pub fn convert_batch<I>(&self, items: I, request: ConversionRequest) -> BatchReceipt
    where
        I: IntoIterator<Item = BatchItem>,
    {
        let mut receipt = BatchReceipt::default();
        for item in items {
            match self.convert_batch_item(&item, request) {
                Ok(success) => receipt.successes.push(success),
                Err(error) => receipt.failures.push(BatchFailure {
                    input: item.input,
                    output: item.output,
                    error: error.to_string(),
                }),
            }
        }
        receipt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_counts_are_derived_from_outcomes() {
        let receipt = BatchReceipt {
            successes: vec![BatchSuccess {
                input: PathBuf::from("a.heic"),
                outputs: vec![PathBuf::from("out/a.heic")],
                kind: BatchAssetKind::ProXdr,
            }],
            failures: vec![BatchFailure {
                input: PathBuf::from("b.heic"),
                output: PathBuf::from("out/b.heic"),
                error: "invalid".to_owned(),
            }],
        };
        assert_eq!(receipt.processed(), 2);
        assert_eq!(receipt.succeeded(), 1);
        assert_eq!(receipt.failed(), 1);
        assert!(!receipt.is_success());
    }
}
