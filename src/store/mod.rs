use std::rc::Rc;

pub mod internment;
pub mod backup;
mod de;

pub type RcStr = Rc<String>;
