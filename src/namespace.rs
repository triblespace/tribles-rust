/*
macro_rules! outer {
    ($mod_name:ident) => {
        pub mod $mod_name {
            #[macro_export]
            macro_rules! inner {
                () => {
                    1
                };
            }
        }
    };
}

outer!(some_mod);
const X: usize = some_mod::entity!();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_namespace() {
        some_ns::entity(1);
    }
}
*/
