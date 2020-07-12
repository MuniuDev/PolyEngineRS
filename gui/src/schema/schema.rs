use polyengine_core::*;

use std::collections::HashMap;
use std::borrow::Cow;

use super::metadata::Metadata;

pub struct Field<'a> {
    pub name: Cow<'a, str>,
    pub field_type: Cow<'a, str>,
    // position: Vector2i,
    // size: Vector2u,
    // metadata: HashMap<Cow<'a, str>, Metadata>,

    // subfields: Vec<Field<'a>>,
}

pub struct Schema<'a> {
    root : Field<'a>
}

impl<'a> Schema<'a> {
    pub fn new(root: Field<'a>) -> Self {
        return Schema{ root };
    }
}
