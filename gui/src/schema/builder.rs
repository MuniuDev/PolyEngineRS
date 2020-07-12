use super::schema::Field; 
use std::borrow::Cow;

pub struct FieldBuilder<'a> {
    name: Cow<'a, str>,
    field_type: &'static str
}

impl<'a> FieldBuilder<'a> {
    pub fn new_button<S>(name: S) -> Self 
    where S : Into<Cow<'a, str>>
    {
        return FieldBuilder{ 
            name : name.into(),
            field_type : "Button"
        }
    }

    // pub fn new_layout() -> Self {
    //     return FieldBuilder{ field : Field }
    // }

    pub fn build(self) -> Field<'a> {
        return Field {
            name: self.name,
            field_type: Cow::Borrowed(self.field_type)
        }
    }
}

// pub struct LayoutBuilder {
//     layout: Field
// }

// impl LayoutBuilder {

// }


// SchemaBuilder::new()
//     .withLayout()
//         .withButton()
//         .withText()
//         .build()
//     .withButton()
//     .build()

