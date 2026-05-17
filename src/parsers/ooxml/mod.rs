pub mod pptx;
pub mod xlsx;
mod xml;

pub use pptx::{parse_pptx_slide_xml, parse_pptx_slide_xml_with_rels};
pub use xlsx::{parse_xlsx_sheet_xml, parse_xlsx_sheet_xml_with_warnings};
