pub mod semantic;
pub mod syntactic;
pub mod handle;

use crate::trible::Value;

pub trait FromValue {
    type Out;

    fn from_value(value: Value) -> Self::Out;
}

pub trait ToValue {
    type In;

    fn to_value(value: &Self::In) -> Value;
}

#[macro_export]
macro_rules! inline_value {
    ($t:ty) => {
        impl $crate::types::FromValue for $t
        where $t: std::convert::From<$crate::trible::Value>,
        {
            type Out = $t;
        
            fn from_value(value: $crate::trible::Value) -> Self::Out {
                value.into()
            }
        }

        impl $crate::types::ToValue for $t
        where for<'a> &'a $t: std::convert::Into<$crate::trible::Value>,
        {
            type In = $t;
        
            fn to_value(value: &Self::In) -> $crate::trible::Value {
                value.into()
            }
        }
    };
}

#[macro_export]
macro_rules! handle_value {
    ($t:ty) => {
        impl $crate::types::FromValue for $t
        where $t: std::convert::From<$crate::trible::Blob>,
        {
            type Out = $crate::types::handle::Handle<$t>;
        
            fn from_value(value: $crate::trible::Value) -> Self::Out {
                $crate::types::handle::Handle::new(value)
            }
        }

        impl $crate::types::ToValue for $t
        where for<'a> &'a $t: std::convert::Into<$crate::trible::Value>,
        {
            type In = $crate::types::handle::Handle<$t>;
        
            fn to_value(handle: &Self::In) -> $crate::trible::Value {
                handle.value
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