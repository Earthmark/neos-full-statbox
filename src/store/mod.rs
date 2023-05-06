pub mod internment;
pub mod backup;
mod de;

pub type CowStr = std::borrow::Cow<'static, str>;
