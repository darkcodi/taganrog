pub mod media;
pub mod tag;

pub use crate::entities::media::Entity as MediaEntity;
pub use crate::entities::media::Column as MediaColumn;
pub use crate::entities::media::Model as Media;
pub use crate::entities::media::ActiveModel as ActiveMedia;

pub use crate::entities::tag::Entity as TagEntity;
pub use crate::entities::tag::Column as TagColumn;
pub use crate::entities::tag::Model as Tag;
pub use crate::entities::tag::ActiveModel as ActiveTag;
