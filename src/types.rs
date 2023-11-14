pub mod semantic;
pub mod syntactic;
pub mod handle;

use crate::trible::Value;

pub trait FromValue {
    type Out;

    fn from_value(value: Value) -> Self::Out;
}

#[macro_export]
macro_rules! inline_value {
    ($t:ty) => {
        impl $crate::types::FromValue for $t
        {
            type Out = $t;
        
            fn from_value(value: $crate::trible::Value) -> Self::Out {
                value.into()
            }
        }
    };
}

#[macro_export]
macro_rules! hash_value {
    ($t:ty) => {
        impl $crate::types::FromValue for $t
        {
            type Out = $crate::types::handle::Handle<$t>;
        
            fn from_value(value: $crate::trible::Value) -> Self::Out {
                $crate::types::handle::Handle::new(value)
            }
        }
    };
}

/*
The day Rust gets off it's macro addiction
and simply starts throwing erros when types collide we can
replace all that cruft with the two impls below...

use crate::trible::{Value, Blob};
use crate::types::handle::Handle;

impl<T> Extract for T
where
    T: for<'a> From<&'a Value>
{
    type Out = T;

    fn extract(value: &Value) -> Self::Out {
        value.into()
    }
}

impl<T> Extract for T
where
    T: From<Blob>
{
    type Out = Handle<T>;

    fn extract(value: &Value) -> Self::Out {
        Handle::new(value)
    }
}
*/