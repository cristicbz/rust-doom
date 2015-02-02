#![feature(slicing_syntax)]
#![feature(std_misc)]
#![feature(core)]

pub use line::Line2;
pub use line::Line2f;
pub use mat::Mat4;
pub use numvec::Numvec;
pub use numvec::Vec2;
pub use numvec::Vec2f;
pub use numvec::Vec3;
pub use numvec::Vec3f;
pub use numvec::Vec4;
pub use numvec::Vec4f;

mod line;
mod mat;
mod numvec;
