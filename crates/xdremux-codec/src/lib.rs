#![forbid(unsafe_code)]

mod error;
mod model;
pub mod portable;

pub use error::{CodecError, Result};
pub use model::{
    EncodedGainMapTiles, EncodedHevcTile, GainMapTileEncodeRequest, HeifRasterDecodeRequest,
    JpegRasterDecodeRequest, PrimaryHeifEncodeRequest, Raster8, RasterPixelFormat,
};
pub use portable::{LibHeifProvider, ZuneJpegProvider};
