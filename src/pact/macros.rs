use super::*;

macro_rules! dispatch {
    ($self:ident, $name:ident, $call:expr) => {
        unsafe {
            match $self.unknown.tag {
                HeadTag::Empty => {
                    let $name = &($self.empty);
                    $call
                }
                HeadTag::Leaf => {
                    let $name = &($self.leaf);
                    $call
                }
                HeadTag::Path14 => {
                    let $name = &($self.path14);
                    $call
                }
                HeadTag::Path30 => {
                    let $name = &($self.path30);
                    $call
                }
                HeadTag::Path46 => {
                    let $name = &($self.path46);
                    $call
                }
                HeadTag::Path62 => {
                    let $name = &($self.path62);
                    $call
                }
                HeadTag::Branch4 => {
                    let $name = &($self.branch4);
                    $call
                }
                HeadTag::Branch8 => {
                    let $name = &($self.branch8);
                    $call
                }
                HeadTag::Branch16 => {
                    let $name = &($self.branch16);
                    $call
                }
                HeadTag::Branch32 => {
                    let $name = &($self.branch32);
                    $call
                }
                HeadTag::Branch64 => {
                    let $name = &($self.branch64);
                    $call
                }
                HeadTag::Branch128 => {
                    let $name = &($self.branch128);
                    $call
                }
                HeadTag::Branch256 => {
                    let $name = &($self.branch256);
                    $call
                }
            }
        }
    };
}

macro_rules! dispatch_mut {
    ($self:ident, $name:ident, $call:expr) => {
        unsafe {
            match $self.unknown.tag {
                HeadTag::Empty => {
                    let $name = &mut ($self.empty);
                    $call
                }
                HeadTag::Leaf => {
                    let $name = &mut ($self.leaf);
                    $call
                }
                HeadTag::Path14 => {
                    let $name = &mut ($self.path14);
                    $call
                }
                HeadTag::Path30 => {
                    let $name = &mut ($self.path30);
                    $call
                }
                HeadTag::Path46 => {
                    let $name = &mut ($self.path46);
                    $call
                }
                HeadTag::Path62 => {
                    let $name = &mut ($self.path62);
                    $call
                }
                HeadTag::Branch4 => {
                    let $name = &mut ($self.branch4);
                    $call
                }
                HeadTag::Branch8 => {
                    let $name = &mut ($self.branch8);
                    $call
                }
                HeadTag::Branch16 => {
                    let $name = &mut ($self.branch16);
                    $call
                }
                HeadTag::Branch32 => {
                    let $name = &mut ($self.branch32);
                    $call
                }
                HeadTag::Branch64 => {
                    let $name = &mut ($self.branch64);
                    $call
                }
                HeadTag::Branch128 => {
                    let $name = &mut ($self.branch128);
                    $call
                }
                HeadTag::Branch256 => {
                    let $name = &mut ($self.branch256);
                    $call
                }
            }
        }
    };
}

pub(super) use dispatch;
pub(super) use dispatch_mut;
