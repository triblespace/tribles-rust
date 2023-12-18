pub mod handle;
pub mod semantic;
pub mod syntactic;

use crate::trible::Value;


pub trait FromValue {
    type Rep;

    fn from_value(value: Value) -> Self::Rep;
}

pub trait ToValue {
    type Rep;

    fn to_value(value: &Self::Rep) -> Value;
}

#[macro_export]
macro_rules! inline_value {
    ($t:ty) => {
        impl $crate::types::FromValue for $t
        where
            $t: std::convert::From<$crate::trible::Value>,
        {
            type Rep = $t;

            fn from_value(value: $crate::trible::Value) -> Self::Rep {
                value.into()
            }
        }

        impl $crate::types::ToValue for $t
        where
            for<'a> &'a $t: std::convert::Into<$crate::trible::Value>,
        {
            type Rep = $t;

            fn to_value(value: &Self::Rep) -> $crate::trible::Value {
                value.into()
            }
        }
    };
}

#[macro_export]
macro_rules! handle_value {
    ($h:ty, $t:ty) => {
        impl $crate::types::FromValue for $t
        where
            $h: digest::Digest,
            $t: std::convert::From<$crate::trible::Blob>,
        {
            type Rep = $crate::types::handle::Handle<$h, $t>;

            fn from_value(value: $crate::trible::Value) -> Self::Rep {
                $crate::types::handle::Handle::new(value)
            }
        }

        impl $crate::types::ToValue for $t
        where
            for<'a> &'a $t: std::convert::Into<$crate::trible::Blob>,
        {
            type Rep = $crate::types::handle::Handle<$h, $t>;

            fn to_value(handle: &Self::Rep) -> $crate::trible::Value {
                handle.hash.value
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
