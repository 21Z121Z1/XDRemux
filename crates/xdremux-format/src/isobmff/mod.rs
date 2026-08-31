mod boxes;
mod construct;
mod model;
mod parser;

pub use boxes::{parse_boxes, parse_irot_quarter_turns, parse_ispe_dimensions, scan_top_level_boxes};
pub use construct::{
    make_box, make_full_box, make_iinf_box, make_iloc_box, make_infe_box, make_ipma_box,
    make_iref_box, make_irot_box, make_ispe_box, make_pitm_box,
};
pub use model::*;
pub use parser::{
    parse_iinf, parse_iloc, parse_ipco_properties, parse_ipma, parse_iref, parse_meta_box,
    parse_pitm,
};

#[cfg(test)]
mod tests;
