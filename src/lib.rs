use stackpin::PinStack;

///
/// # Safety
///
/// * Implementers **must** write a valid `Self` to the `dst` argument of `transfer`
/// * Implementers are **not** allowed to panic in the `transfer` function
/// * Implementers **must** reset `pin` to a value that can be safely dropped without incidence on
///   the `dst` pointer that was written to in the `transfer` function
pub unsafe trait Transfer {
    /// # Safety
    ///
    /// * Callers of this function **must** call `reset` on the `src` argument right afterwards.
    /// * `dst` must point to a `Self` instance, that can possibly be uninitialized
    /// * `src` and `dest` **must** point to different instances.
    unsafe fn transfer(src: &mut PinStack<'_, Self>, dst: *mut Self)
    where
        Self: Sized;

    fn empty() -> Tr<Self>;
}

pub struct Tr<T: ?Sized>(T);

impl<T: Transfer> Tr<T> {
    pub fn from_empty(empty: T) -> Self {
        Self(empty)
    }

    fn slot(&mut self) -> *mut T {
        &mut self.0 as *mut T
    }
}

pub fn transfer<'old, 'new, T>(
    mut src: PinStack<'old, T>,
    dest: &'new mut Tr<T>,
) -> PinStack<'new, T>
where
    T: Transfer,
{
    use stackpin::StackPinned;
    use std::pin::Pin;
    unsafe {
        let slot = dest.slot();
        T::transfer(&mut src, slot);
        Pin::new_unchecked(StackPinned::new(&mut *slot))
    }
}

#[macro_export]
macro_rules! transfer_let {
    ($id:ident = $fun_name:ident ($($arg:expr),*)) => {
        let mut $id = $crate::Transfer::empty();
        let $id = $fun_name($($arg),* &mut $id);
    };
    ($id:ident = $e:expr) => {
        let mut $id = $crate::Transfer::empty();
        let $id = $crate::transfer($e, &mut $id);
    };
}

#[cfg(test)]
mod tests {

    mod secret {
        use std::marker::PhantomPinned;
        pub struct SecretU64(u64, PhantomPinned);

        fn secure_erase(x: &mut u64) {
            *x = 0;
        }

        use super::super::{Tr, Transfer};
        use stackpin::FromUnpinned;
        use stackpin::PinStack;

        unsafe impl<'a> FromUnpinned<&'a mut u64> for SecretU64 {
            type PinData = &'a mut u64;

            unsafe fn from_unpinned(src: &'a mut u64) -> (Self, &'a mut u64) {
                (Self(0, PhantomPinned), src)
            }

            unsafe fn on_pin(&mut self, data: &'a mut u64) {
                self.0 = *data;
                println!(
                    "Secure erasing data that served for construction at {:p}",
                    data
                );
                secure_erase(data);
            }
        }

        unsafe impl Transfer for SecretU64 {
            unsafe fn transfer(src: &mut PinStack<'_, Self>, dst: *mut Self) {
                (*dst).0 = src.0;
                secure_erase(&mut src.as_mut().get_unchecked_mut().0);
                println!(
                    "Secure erasing on transfer for {:p}",
                    &mut src.as_mut().get_unchecked_mut().0
                );
            }

            fn empty() -> Tr<Self> {
                Tr::from_empty(Self(0, PhantomPinned))
            }
        }

        impl SecretU64 {
            pub fn reveal(this: &PinStack<'_, Self>) -> u64 {
                this.0
            }
        }

        impl Drop for SecretU64 {
            fn drop(&mut self) {
                if self.0 == 0 {
                    println!("Not erasing empty secret at {:p}", self);
                } else {
                    println!("Secure erasing in dtor for {:p}", self);
                    secure_erase(&mut self.0)
                }
            }
        }

        pub fn generate_secret(slot: &mut crate::Tr<SecretU64>) -> PinStack<'_, SecretU64> {
            let mut secret = 42;
            stackpin::stack_let!(secret = stackpin::Unpinned::new(&mut secret));
            crate::transfer(secret, slot)
        }
    }

    use secret::SecretU64;

    #[test]
    fn outin_transfer() {
        use secret::generate_secret;
        super::transfer_let!(my_secret = generate_secret());
        assert_eq!(SecretU64::reveal(&my_secret), 42);
    }

    fn transfer_secret(outer_secret: stackpin::PinStack<'_, secret::SecretU64>) {
        super::transfer_let!(inner_secret = outer_secret);
        assert_eq!(SecretU64::reveal(&inner_secret), 83);
    }

    #[test]
    fn inout_transfer() {
        let mut initial_secret = 83u64;
        stackpin::stack_let!(my_secret: SecretU64 = &mut initial_secret);
        transfer_secret(my_secret);
        assert_eq!(initial_secret, 0);
    }
}
