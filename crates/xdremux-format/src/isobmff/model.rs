use std::ops::Range;

use crate::error::{FormatError, Result};
use crate::fourcc::FourCC;

pub const FTYP: FourCC = FourCC::new(*b"ftyp");
pub const META: FourCC = FourCC::new(*b"meta");
pub const MDAT: FourCC = FourCC::new(*b"mdat");
pub const ILOC: FourCC = FourCC::new(*b"iloc");
pub const IINF: FourCC = FourCC::new(*b"iinf");
pub const INFE: FourCC = FourCC::new(*b"infe");
pub const PITM: FourCC = FourCC::new(*b"pitm");
pub const IPRP: FourCC = FourCC::new(*b"iprp");
pub const IPCO: FourCC = FourCC::new(*b"ipco");
pub const IPMA: FourCC = FourCC::new(*b"ipma");
pub const IREF: FourCC = FourCC::new(*b"iref");
pub const IDAT: FourCC = FourCC::new(*b"idat");
pub const ISPE: FourCC = FourCC::new(*b"ispe");
pub const IROT: FourCC = FourCC::new(*b"irot");
pub const EXIF: FourCC = FourCC::new(*b"Exif");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxHeader {
    pub kind: FourCC,
    pub box_start: usize,
    pub data_start: usize,
    pub data_end: usize,
    pub size: usize,
}

impl BoxHeader {
    pub fn box_range(&self) -> Range<usize> {
        self.box_start..self.data_end
    }

    pub fn payload_range(&self) -> Range<usize> {
        self.data_start..self.data_end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopLevelScan {
    pub boxes: Vec<BoxHeader>,
    pub trailing_range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlocExtent {
    pub index: Option<u64>,
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlocEntry {
    pub item_id: u32,
    pub construction_method: u16,
    pub data_reference_index: u16,
    pub base_offset: u64,
    pub extents: Vec<IlocExtent>,
}

impl IlocEntry {
    pub fn resolved_extent_offset(&self, extent: &IlocExtent) -> Result<u64> {
        self.base_offset
            .checked_add(extent.offset)
            .ok_or_else(|| FormatError::overflow("iloc extent offset"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlocBox {
    pub version: u8,
    pub offset_size: u8,
    pub length_size: u8,
    pub base_offset_size: u8,
    pub index_size: u8,
    pub entries: Vec<IlocEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInfo {
    pub item_id: u32,
    pub item_type: Option<FourCC>,
    pub flags: u32,
    pub box_range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IinfBox {
    pub version: u8,
    pub entries: Vec<ItemInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpmaAssociation {
    pub property_index: u16,
    pub essential: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpmaEntry {
    pub item_id: u32,
    pub associations: Vec<IpmaAssociation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpmaBox {
    pub version: u8,
    pub flags: u32,
    pub entries: Vec<IpmaEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrefEntry {
    pub kind: FourCC,
    pub from_item_id: u32,
    pub to_item_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrefBox {
    pub version: u8,
    pub entries: Vec<IrefEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyInfo {
    pub index: u32,
    pub kind: FourCC,
    pub box_range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMeta {
    pub iloc: IlocBox,
    pub iinf: IinfBox,
    pub primary_item_id: u32,
    pub ipma: IpmaBox,
    pub properties: Vec<PropertyInfo>,
    pub iref: Option<IrefBox>,
    pub idat: Option<BoxHeader>,
}
