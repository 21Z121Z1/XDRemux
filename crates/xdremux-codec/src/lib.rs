#![forbid(unsafe_code)]

mod error;
mod model;
pub mod portable;

pub use error::{CodecError, Result};
pub use model::{
    undo_heif_rgb16, EncodedGainMapTiles, EncodedHevcResource, EncodedHevcTile,
    GainMapTileEncodeRequest, HeifRasterDecodeRequest, JpegRasterDecodeRequest,
    PrimaryHeifEncodeRequest, Raster8, RasterPixelFormat,
};
pub use portable::{LibHeifProvider, ZuneJpegProvider};
